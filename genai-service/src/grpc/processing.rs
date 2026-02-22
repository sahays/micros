//! gRPC handlers for AI processing operations (unary and streaming).

use crate::config::OutputFormat;
use crate::grpc::helpers::{
    build_generation_params, build_stream_generation_params, finish_reason_to_proto,
    output_format_str, output_format_to_proto, proto_to_document_context, proto_to_output_format,
    provider_error_to_status,
};
use crate::grpc::proto::{
    ProcessRequest, ProcessResponse, ProcessStreamRequest, ProcessStreamResponse, StreamComplete,
    TokenUsage,
};
use crate::models::UsageRecord;
use crate::services::document_fetcher::TenantContext;
use crate::services::providers::{DocumentContext, StreamChunk};
use crate::startup::AppState;
use futures::StreamExt;
use std::time::Instant;
use tonic::{Request, Response, Status};

use super::capability_check::extract_org_node_id;
use super::genai_service::ProcessStreamResult;

#[tracing::instrument(
    skip(state, request),
    fields(
        request_id,
        tenant_id,
        user_id,
        model,
        output_format,
        prompt_len,
        doc_count
    )
)]
pub async fn process(
    state: &AppState,
    request: Request<ProcessRequest>,
) -> Result<Response<ProcessResponse>, Status> {
    let start = Instant::now();

    let auth = state
        .capability_checker
        .require_capability(
            &request,
            crate::grpc::capability_check::capabilities::GENAI_PROCESS,
        )
        .await?;

    // Extract org_id from request metadata before consuming the request
    let org_id = match extract_org_node_id(&request) {
        Some(id) => id,
        None => {
            tracing::warn!(
                tenant_id = %auth.tenant_id,
                "Missing org_id in request metadata (x-org-id header), defaulting to empty string"
            );
            String::new()
        }
    };

    let req = request.into_inner();
    let request_id = uuid::Uuid::new_v4().to_string();

    // Build tenant context for document-service calls
    let tenant_context = TenantContext {
        app_id: auth.tenant_id.clone(),
        org_id,
        user_id: auth.user_id.clone(),
    };

    let span = tracing::Span::current();
    span.record("request_id", &request_id);
    span.record("tenant_id", &auth.tenant_id);
    span.record("user_id", &auth.user_id);
    span.record("prompt_len", req.prompt.len());
    span.record("doc_count", req.documents.len());

    if req.prompt.is_empty() {
        tracing::warn!(request_id = %request_id, "Empty prompt rejected");
        return Err(Status::invalid_argument("Prompt is required"));
    }

    let output_format = proto_to_output_format(req.output_format);
    span.record("output_format", output_format_str(output_format));

    if output_format == OutputFormat::StructuredJson {
        match &req.output_schema {
            None => {
                tracing::warn!(request_id = %request_id, "Missing output_schema for STRUCTURED_JSON");
                return Err(Status::invalid_argument(
                    "output_schema is required for STRUCTURED_JSON output format",
                ));
            }
            Some(schema) => {
                if serde_json::from_str::<serde_json::Value>(schema).is_err() {
                    tracing::warn!(request_id = %request_id, "Invalid JSON schema");
                    return Err(Status::invalid_argument("output_schema must be valid JSON"));
                }
            }
        }
    }

    let model_override = req
        .model
        .as_deref()
        .filter(|m| !m.is_empty())
        .map(String::from);

    if let Some(ref requested_model) = model_override {
        if !state.config.is_valid_model(requested_model) {
            let valid = state.config.valid_models().join(", ");
            tracing::warn!(
                request_id = %request_id,
                requested_model = %requested_model,
                "Unknown model requested"
            );
            return Err(Status::invalid_argument(format!(
                "Unknown model '{}'. Valid models: {}",
                requested_model, valid
            )));
        }
    }

    let model = model_override
        .clone()
        .unwrap_or_else(|| state.config.model_for_output(output_format).to_string());
    span.record("model", &model);

    tracing::info!(request_id = %request_id, "Processing gRPC request");

    let documents: Vec<DocumentContext> = req
        .documents
        .iter()
        .map(proto_to_document_context)
        .collect();

    let enriched_documents = match state
        .document_fetcher
        .enrich_documents(&documents, &tenant_context)
        .await
    {
        Ok(docs) => docs,
        Err(e) => {
            tracing::warn!(
                request_id = %request_id,
                error = %e,
                "Failed to enrich documents, using original context"
            );
            documents
        }
    };

    let mut params = build_generation_params(&req, output_format, model_override);
    params.tenant_context = Some(tenant_context);

    let provider_response = match state
        .text_provider
        .generate(&req.prompt, &enriched_documents, &params)
        .await
    {
        Ok(response) => response,
        Err(e) => {
            let status = provider_error_to_status(e);
            return Err(status);
        }
    };

    let result = match output_format {
        OutputFormat::Text => provider_response
            .text
            .map(crate::grpc::proto::process_response::Result::Text),
        OutputFormat::StructuredJson => provider_response
            .text
            .map(crate::grpc::proto::process_response::Result::Json),
    };

    let usage = TokenUsage {
        input_tokens: provider_response.input_tokens,
        output_tokens: provider_response.output_tokens,
        total_tokens: provider_response.input_tokens + provider_response.output_tokens,
    };

    let duration = start.elapsed();

    tracing::info!(
        request_id = %request_id,
        input_tokens = provider_response.input_tokens,
        output_tokens = provider_response.output_tokens,
        duration_ms = duration.as_millis(),
        finish_reason = ?provider_response.finish_reason,
        "Request completed successfully"
    );

    {
        let tags = req
            .metadata
            .as_ref()
            .map(|m| m.tags.clone())
            .unwrap_or_default();
        let usage_record = UsageRecord::new(
            request_id.clone(),
            req.session_id.clone(),
            auth.tenant_id.clone(),
            auth.user_id.clone(),
            model.clone(),
            provider_response.input_tokens,
            provider_response.output_tokens,
            output_format_str(output_format).to_string(),
            tags,
        );

        if let Err(e) = state.db.record_usage(&usage_record).await {
            tracing::warn!(
                request_id = %request_id,
                error = %e,
                "Failed to record usage (non-critical)"
            );
        }
    }

    let response = ProcessResponse {
        result,
        output_format: output_format_to_proto(output_format),
        model,
        usage: Some(usage),
        finish_reason: finish_reason_to_proto(provider_response.finish_reason),
        request_id,
        session_id: req.session_id,
    };

    Ok(Response::new(response))
}

#[tracing::instrument(
    skip(state, request),
    fields(
        request_id,
        tenant_id,
        user_id,
        model,
        output_format,
        prompt_len,
        doc_count
    )
)]
pub async fn process_stream(
    state: &AppState,
    request: Request<ProcessStreamRequest>,
) -> Result<Response<ProcessStreamResult>, Status> {
    let start = Instant::now();

    let auth = state
        .capability_checker
        .require_capability(
            &request,
            crate::grpc::capability_check::capabilities::GENAI_PROCESS,
        )
        .await?;

    // Extract org_id from request metadata before consuming the request
    let org_id = match extract_org_node_id(&request) {
        Some(id) => id,
        None => {
            tracing::warn!(
                tenant_id = %auth.tenant_id,
                "Missing org_id in request metadata (x-org-id header), defaulting to empty string"
            );
            String::new()
        }
    };

    let req = request.into_inner();
    let request_id = uuid::Uuid::new_v4().to_string();

    // Build tenant context for document-service calls
    let tenant_context = TenantContext {
        app_id: auth.tenant_id.clone(),
        org_id,
        user_id: auth.user_id.clone(),
    };

    let span = tracing::Span::current();
    span.record("request_id", &request_id);
    span.record("tenant_id", &auth.tenant_id);
    span.record("user_id", &auth.user_id);
    span.record("prompt_len", req.prompt.len());
    span.record("doc_count", req.documents.len());

    if req.prompt.is_empty() {
        tracing::warn!(request_id = %request_id, "Empty prompt rejected");
        return Err(Status::invalid_argument("Prompt is required"));
    }

    let output_format = proto_to_output_format(req.output_format);
    span.record("output_format", output_format_str(output_format));

    if output_format == OutputFormat::StructuredJson {
        match &req.output_schema {
            None => {
                tracing::warn!(request_id = %request_id, "Missing output_schema for STRUCTURED_JSON");
                return Err(Status::invalid_argument(
                    "output_schema is required for STRUCTURED_JSON output format",
                ));
            }
            Some(schema) => {
                if serde_json::from_str::<serde_json::Value>(schema).is_err() {
                    tracing::warn!(request_id = %request_id, "Invalid JSON schema");
                    return Err(Status::invalid_argument("output_schema must be valid JSON"));
                }
            }
        }
    }

    let model_override = req
        .model
        .as_deref()
        .filter(|m| !m.is_empty())
        .map(String::from);

    if let Some(ref requested_model) = model_override {
        if !state.config.is_valid_model(requested_model) {
            let valid = state.config.valid_models().join(", ");
            tracing::warn!(
                request_id = %request_id,
                requested_model = %requested_model,
                "Unknown model requested"
            );
            return Err(Status::invalid_argument(format!(
                "Unknown model '{}'. Valid models: {}",
                requested_model, valid
            )));
        }
    }

    let model = model_override
        .clone()
        .unwrap_or_else(|| state.config.model_for_output(output_format).to_string());
    span.record("model", &model);

    tracing::info!(request_id = %request_id, "Starting streaming gRPC request");

    let documents: Vec<DocumentContext> = req
        .documents
        .iter()
        .map(proto_to_document_context)
        .collect();

    let enriched_documents = match state
        .document_fetcher
        .enrich_documents(&documents, &tenant_context)
        .await
    {
        Ok(docs) => docs,
        Err(e) => {
            tracing::warn!(
                request_id = %request_id,
                error = %e,
                "Failed to enrich documents, using original context"
            );
            documents
        }
    };

    let mut params = build_stream_generation_params(&req, output_format, model_override);
    params.tenant_context = Some(tenant_context);

    let mut provider_stream = match state
        .text_provider
        .generate_stream(&req.prompt, &enriched_documents, &params)
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            let status = provider_error_to_status(e);
            return Err(status);
        }
    };

    let connect_duration = start.elapsed();
    tracing::debug!(
        request_id = %request_id,
        duration_ms = connect_duration.as_millis(),
        "Stream connection established"
    );

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    let session_id = req.session_id.clone();
    let model_clone = model.clone();
    let request_id_clone = request_id.clone();
    let output_format_clone = output_format;

    tokio::spawn(async move {
        let stream_start = Instant::now();
        let mut chunk_count = 0u32;

        while let Some(chunk_result) = provider_stream.next().await {
            let grpc_response = match chunk_result {
                Ok(chunk) => match chunk {
                    StreamChunk::Text(text) => {
                        chunk_count += 1;
                        Ok(ProcessStreamResponse {
                            data: Some(
                                crate::grpc::proto::process_stream_response::Data::TextChunk(text),
                            ),
                        })
                    }
                    StreamChunk::Complete {
                        input_tokens,
                        output_tokens,
                        finish_reason,
                    } => {
                        let stream_duration = stream_start.elapsed();

                        tracing::info!(
                            request_id = %request_id_clone,
                            input_tokens = input_tokens,
                            output_tokens = output_tokens,
                            chunk_count = chunk_count,
                            duration_ms = stream_duration.as_millis(),
                            finish_reason = ?finish_reason,
                            "Streaming request completed"
                        );

                        let complete = StreamComplete {
                            output_format: output_format_to_proto(output_format_clone),
                            model: model_clone.clone(),
                            usage: Some(TokenUsage {
                                input_tokens,
                                output_tokens,
                                total_tokens: input_tokens + output_tokens,
                            }),
                            finish_reason: finish_reason_to_proto(finish_reason),
                            request_id: request_id_clone.clone(),
                            session_id: session_id.clone(),
                        };

                        Ok(ProcessStreamResponse {
                            data: Some(
                                crate::grpc::proto::process_stream_response::Data::Complete(
                                    complete,
                                ),
                            ),
                        })
                    }
                },
                Err(e) => {
                    tracing::error!(
                        request_id = %request_id_clone,
                        error = %e,
                        "Error in provider stream"
                    );
                    Err(provider_error_to_status(e))
                }
            };

            if tx.send(grpc_response).await.is_err() {
                tracing::debug!(
                    request_id = %request_id_clone,
                    "Client disconnected, stopping stream"
                );
                break;
            }
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok(Response::new(Box::pin(stream) as ProcessStreamResult))
}
