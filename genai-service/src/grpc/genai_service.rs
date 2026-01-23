use crate::config::OutputFormat;
use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::proto::{
    gen_ai_service_server::GenAiService, CreateSessionRequest, CreateSessionResponse,
    DeleteSessionRequest, DeleteSessionResponse, FinishReason, GetSessionRequest,
    GetSessionResponse, GetUsageRequest, GetUsageResponse, ListModelsRequest, ListModelsResponse,
    ModelInfo, OutputFormat as ProtoOutputFormat, ProcessRequest, ProcessResponse,
    ProcessStreamRequest, ProcessStreamResponse, Session as ProtoSession,
    SessionMessage as ProtoSessionMessage, StreamComplete, TokenUsage,
};
use crate::models::{Session, SessionDocument, SessionMessage, UsageRecord};
use crate::services::metrics::{
    dec_grpc_in_flight, inc_grpc_in_flight, record_genai_request, record_grpc_request,
    record_tokens,
};
use crate::services::providers::{
    DocumentContext, FinishReason as ProviderFinishReason, GenerationParams, ProviderError,
    StreamChunk,
};
use crate::startup::AppState;
use chrono::Utc;
use futures::{Stream, StreamExt};
use prost_types::Timestamp;
use std::pin::Pin;
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct GenaiGrpcService {
    state: AppState,
}

impl GenaiGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert proto output format to internal enum.
fn proto_to_output_format(format: i32) -> OutputFormat {
    match format {
        1 => OutputFormat::Text,
        2 => OutputFormat::StructuredJson,
        3 => OutputFormat::Audio,
        4 => OutputFormat::Video,
        _ => OutputFormat::Text,
    }
}

/// Convert internal output format to proto enum.
fn output_format_to_proto(format: OutputFormat) -> i32 {
    match format {
        OutputFormat::Text => ProtoOutputFormat::Text as i32,
        OutputFormat::StructuredJson => ProtoOutputFormat::StructuredJson as i32,
        OutputFormat::Audio => ProtoOutputFormat::Audio as i32,
        OutputFormat::Video => ProtoOutputFormat::Video as i32,
    }
}

/// Convert output format to string for metrics.
fn output_format_str(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Text => "text",
        OutputFormat::StructuredJson => "json",
        OutputFormat::Audio => "audio",
        OutputFormat::Video => "video",
    }
}

/// Convert provider finish reason to proto enum.
fn finish_reason_to_proto(reason: ProviderFinishReason) -> i32 {
    match reason {
        ProviderFinishReason::Complete => FinishReason::Complete as i32,
        ProviderFinishReason::Length => FinishReason::Length as i32,
        ProviderFinishReason::ContentFilter => FinishReason::ContentFilter as i32,
        ProviderFinishReason::Error => FinishReason::Error as i32,
    }
}

/// Convert finish reason to string for metrics.
fn finish_reason_str(reason: ProviderFinishReason) -> &'static str {
    match reason {
        ProviderFinishReason::Complete => "complete",
        ProviderFinishReason::Length => "length",
        ProviderFinishReason::ContentFilter => "content_filter",
        ProviderFinishReason::Error => "error",
    }
}

/// Convert provider error to gRPC status with error classification.
fn provider_error_to_status(error: ProviderError) -> Status {
    let (code, message, error_type) = match &error {
        ProviderError::NotConfigured(msg) => (
            tonic::Code::FailedPrecondition,
            msg.clone(),
            "not_configured",
        ),
        ProviderError::ApiError(msg) => (
            tonic::Code::Internal,
            format!("Provider API error: {}", msg),
            "api_error",
        ),
        ProviderError::InvalidRequest(msg) => {
            (tonic::Code::InvalidArgument, msg.clone(), "invalid_request")
        }
        ProviderError::RateLimited => (
            tonic::Code::ResourceExhausted,
            "Rate limited by AI provider".to_string(),
            "rate_limited",
        ),
        ProviderError::ContentFiltered => (
            tonic::Code::InvalidArgument,
            "Content was filtered by AI provider safety settings".to_string(),
            "content_filtered",
        ),
        ProviderError::NetworkError(msg) => (
            tonic::Code::Unavailable,
            format!("Network error: {}", msg),
            "network_error",
        ),
    };

    tracing::error!(
        error_type = error_type,
        grpc_code = ?code,
        error = %error,
        "Provider error converted to gRPC status"
    );

    Status::new(code, message)
}

/// Convert proto document context to provider document context.
fn proto_to_document_context(doc: &crate::grpc::proto::DocumentContext) -> DocumentContext {
    DocumentContext {
        document_id: doc.document_id.clone(),
        url: doc.signed_url.clone(),
        mime_type: doc.mime_type.clone(),
        text_content: doc.text_content.clone(),
    }
}

/// Build generation params from request.
fn build_generation_params(req: &ProcessRequest, output_format: OutputFormat) -> GenerationParams {
    let params = req.params.as_ref();

    GenerationParams {
        temperature: params.and_then(|p| p.temperature),
        top_p: params.and_then(|p| p.top_p),
        max_tokens: None, // Auto-determined based on content size
        stop_sequences: params.map(|p| p.stop_sequences.clone()).unwrap_or_default(),
        output_schema: if output_format == OutputFormat::StructuredJson {
            req.output_schema.clone()
        } else {
            None
        },
        voice: params.and_then(|p| p.voice.clone()),
        audio_format: params.and_then(|p| p.audio_format.clone()),
        video_format: params.and_then(|p| p.video_format.clone()),
        duration_seconds: params.and_then(|p| p.duration_seconds),
    }
}

/// Build generation params from stream request.
fn build_stream_generation_params(
    req: &ProcessStreamRequest,
    output_format: OutputFormat,
) -> GenerationParams {
    let params = req.params.as_ref();

    GenerationParams {
        temperature: params.and_then(|p| p.temperature),
        top_p: params.and_then(|p| p.top_p),
        max_tokens: None, // Auto-determined based on content size
        stop_sequences: params.map(|p| p.stop_sequences.clone()).unwrap_or_default(),
        output_schema: if output_format == OutputFormat::StructuredJson {
            req.output_schema.clone()
        } else {
            None
        },
        voice: params.and_then(|p| p.voice.clone()),
        audio_format: params.and_then(|p| p.audio_format.clone()),
        video_format: params.and_then(|p| p.video_format.clone()),
        duration_seconds: params.and_then(|p| p.duration_seconds),
    }
}

/// Convert chrono DateTime to protobuf Timestamp.
fn datetime_to_timestamp(dt: chrono::DateTime<Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

/// Convert Session model to proto Session.
fn session_to_proto(session: &Session) -> ProtoSession {
    ProtoSession {
        id: session.session_id.clone(),
        title: session.title.clone(),
        system_prompt: session.system_prompt.clone(),
        documents: session
            .documents
            .iter()
            .map(|d| crate::grpc::proto::DocumentContext {
                document_id: d.document_id.clone(),
                signed_url: d.signed_url.clone(),
                mime_type: d.mime_type.clone(),
                text_content: d.text_content.clone(),
            })
            .collect(),
        message_count: session.message_count,
        total_usage: Some(TokenUsage {
            input_tokens: session.total_input_tokens,
            output_tokens: session.total_output_tokens,
            total_tokens: session.total_input_tokens + session.total_output_tokens,
        }),
        created_at: datetime_to_timestamp(session.created_at),
        updated_at: datetime_to_timestamp(session.updated_at),
    }
}

/// Convert SessionMessage model to proto SessionMessage.
fn session_message_to_proto(msg: &SessionMessage) -> ProtoSessionMessage {
    ProtoSessionMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
        output_format: msg
            .output_format
            .as_ref()
            .map(|f| match f.as_str() {
                "text" => ProtoOutputFormat::Text as i32,
                "json" => ProtoOutputFormat::StructuredJson as i32,
                "audio" => ProtoOutputFormat::Audio as i32,
                "video" => ProtoOutputFormat::Video as i32,
                _ => ProtoOutputFormat::Unspecified as i32,
            })
            .unwrap_or(ProtoOutputFormat::Unspecified as i32),
        timestamp: datetime_to_timestamp(msg.timestamp),
    }
}

type ProcessStreamResult =
    Pin<Box<dyn Stream<Item = Result<ProcessStreamResponse, Status>> + Send>>;

#[tonic::async_trait]
impl GenAiService for GenaiGrpcService {
    type ProcessStreamStream = ProcessStreamResult;

    #[tracing::instrument(
        skip(self, request),
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
    async fn process(
        &self,
        request: Request<ProcessRequest>,
    ) -> Result<Response<ProcessResponse>, Status> {
        let start = Instant::now();
        let method = "Process";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_PROCESS)
                .await?;
        }

        let req = request.into_inner();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Record span fields
        let span = tracing::Span::current();
        span.record("request_id", &request_id);
        if let Some(ref metadata) = req.metadata {
            span.record("tenant_id", &metadata.tenant_id);
            span.record("user_id", &metadata.user_id);
        }
        span.record("prompt_len", req.prompt.len());
        span.record("doc_count", req.documents.len());

        // Validate request
        if req.prompt.is_empty() {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!(request_id = %request_id, "Empty prompt rejected");
            return Err(Status::invalid_argument("Prompt is required"));
        }

        let output_format = proto_to_output_format(req.output_format);
        span.record("output_format", output_format_str(output_format));

        // For structured JSON, schema is required and must be valid JSON
        if output_format == OutputFormat::StructuredJson {
            match &req.output_schema {
                None => {
                    dec_grpc_in_flight(method);
                    record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
                    tracing::warn!(request_id = %request_id, "Missing output_schema for STRUCTURED_JSON");
                    return Err(Status::invalid_argument(
                        "output_schema is required for STRUCTURED_JSON output format",
                    ));
                }
                Some(schema) => {
                    // Validate that the schema is valid JSON
                    if serde_json::from_str::<serde_json::Value>(schema).is_err() {
                        dec_grpc_in_flight(method);
                        record_grpc_request(
                            method,
                            "INVALID_ARGUMENT",
                            start.elapsed().as_secs_f64(),
                        );
                        tracing::warn!(request_id = %request_id, "Invalid JSON schema");
                        return Err(Status::invalid_argument("output_schema must be valid JSON"));
                    }
                }
            }
        }

        // Get the appropriate model based on output format
        let model = self
            .state
            .config
            .model_for_output(output_format)
            .to_string();
        span.record("model", &model);

        tracing::info!(
            request_id = %request_id,
            "Processing gRPC request"
        );

        // Convert documents to provider format
        let documents: Vec<DocumentContext> = req
            .documents
            .iter()
            .map(proto_to_document_context)
            .collect();

        // Enrich documents with extracted text from document-service
        let enriched_documents = match self
            .state
            .document_fetcher
            .enrich_documents(&documents)
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

        // Build generation params
        let params = build_generation_params(&req, output_format);

        // Call the provider
        let provider_response = match self
            .state
            .text_provider
            .generate(&req.prompt, &enriched_documents, &params)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                dec_grpc_in_flight(method);
                let status = provider_error_to_status(e);
                record_grpc_request(
                    method,
                    status.code().description(),
                    start.elapsed().as_secs_f64(),
                );
                return Err(status);
            }
        };

        // Build response based on output format
        let result = match output_format {
            OutputFormat::Text => provider_response
                .text
                .map(crate::grpc::proto::process_response::Result::Text),
            OutputFormat::StructuredJson => provider_response
                .text
                .map(crate::grpc::proto::process_response::Result::Json),
            OutputFormat::Audio => provider_response
                .audio
                .map(crate::grpc::proto::process_response::Result::Audio),
            OutputFormat::Video => provider_response
                .video
                .map(crate::grpc::proto::process_response::Result::Video),
        };

        let usage = TokenUsage {
            input_tokens: provider_response.input_tokens,
            output_tokens: provider_response.output_tokens,
            total_tokens: provider_response.input_tokens + provider_response.output_tokens,
        };

        // Record metrics with tenant_id for billing
        let tenant_id = req
            .metadata
            .as_ref()
            .map(|m| m.tenant_id.as_str())
            .unwrap_or("unknown");
        record_tokens(
            tenant_id,
            &model,
            provider_response.input_tokens,
            provider_response.output_tokens,
        );
        record_genai_request(
            tenant_id,
            output_format_str(output_format),
            &model,
            finish_reason_str(provider_response.finish_reason),
        );

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::info!(
            request_id = %request_id,
            input_tokens = provider_response.input_tokens,
            output_tokens = provider_response.output_tokens,
            duration_ms = duration.as_millis(),
            finish_reason = ?provider_response.finish_reason,
            "Request completed successfully"
        );

        // Record usage
        if let Some(ref metadata) = req.metadata {
            let usage_record = UsageRecord::new(
                request_id.clone(),
                req.session_id.clone(),
                metadata.tenant_id.clone(),
                metadata.user_id.clone(),
                model.clone(),
                provider_response.input_tokens,
                provider_response.output_tokens,
                output_format_str(output_format).to_string(),
                metadata.tags.clone(),
            );

            if let Err(e) = self.state.db.record_usage(&usage_record).await {
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
        skip(self, request),
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
    async fn process_stream(
        &self,
        request: Request<ProcessStreamRequest>,
    ) -> Result<Response<Self::ProcessStreamStream>, Status> {
        let start = Instant::now();
        let method = "ProcessStream";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_PROCESS)
                .await?;
        }

        let req = request.into_inner();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Record span fields
        let span = tracing::Span::current();
        span.record("request_id", &request_id);
        if let Some(ref metadata) = req.metadata {
            span.record("tenant_id", &metadata.tenant_id);
            span.record("user_id", &metadata.user_id);
        }
        span.record("prompt_len", req.prompt.len());
        span.record("doc_count", req.documents.len());

        // Validate request
        if req.prompt.is_empty() {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!(request_id = %request_id, "Empty prompt rejected");
            return Err(Status::invalid_argument("Prompt is required"));
        }

        let output_format = proto_to_output_format(req.output_format);
        span.record("output_format", output_format_str(output_format));

        // For structured JSON, schema is required and must be valid JSON
        if output_format == OutputFormat::StructuredJson {
            match &req.output_schema {
                None => {
                    dec_grpc_in_flight(method);
                    record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
                    tracing::warn!(request_id = %request_id, "Missing output_schema for STRUCTURED_JSON");
                    return Err(Status::invalid_argument(
                        "output_schema is required for STRUCTURED_JSON output format",
                    ));
                }
                Some(schema) => {
                    if serde_json::from_str::<serde_json::Value>(schema).is_err() {
                        dec_grpc_in_flight(method);
                        record_grpc_request(
                            method,
                            "INVALID_ARGUMENT",
                            start.elapsed().as_secs_f64(),
                        );
                        tracing::warn!(request_id = %request_id, "Invalid JSON schema");
                        return Err(Status::invalid_argument("output_schema must be valid JSON"));
                    }
                }
            }
        }

        let model = self
            .state
            .config
            .model_for_output(output_format)
            .to_string();
        span.record("model", &model);

        tracing::info!(
            request_id = %request_id,
            "Starting streaming gRPC request"
        );

        // Convert documents to provider format
        let documents: Vec<DocumentContext> = req
            .documents
            .iter()
            .map(proto_to_document_context)
            .collect();

        // Enrich documents with extracted text from document-service
        let enriched_documents = match self
            .state
            .document_fetcher
            .enrich_documents(&documents)
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

        // Build generation params
        let params = build_stream_generation_params(&req, output_format);

        // Get streaming response from provider
        let mut provider_stream = match self
            .state
            .text_provider
            .generate_stream(&req.prompt, &enriched_documents, &params)
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                dec_grpc_in_flight(method);
                let status = provider_error_to_status(e);
                record_grpc_request(
                    method,
                    status.code().description(),
                    start.elapsed().as_secs_f64(),
                );
                return Err(status);
            }
        };

        // Record initial connection time
        let connect_duration = start.elapsed();
        tracing::debug!(
            request_id = %request_id,
            duration_ms = connect_duration.as_millis(),
            "Stream connection established"
        );

        // Create channel for streaming responses
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Clone values for the spawned task
        let session_id = req.session_id.clone();
        let model_clone = model.clone();
        let request_id_clone = request_id.clone();
        let output_format_clone = output_format;
        let tenant_id_clone = req
            .metadata
            .as_ref()
            .map(|m| m.tenant_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Spawn task to transform provider stream to gRPC stream
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
                                    crate::grpc::proto::process_stream_response::Data::TextChunk(
                                        text,
                                    ),
                                ),
                            })
                        }
                        StreamChunk::Audio(audio) => {
                            chunk_count += 1;
                            Ok(ProcessStreamResponse {
                                data: Some(
                                    crate::grpc::proto::process_stream_response::Data::AudioChunk(
                                        audio,
                                    ),
                                ),
                            })
                        }
                        StreamChunk::Video(video) => {
                            chunk_count += 1;
                            Ok(ProcessStreamResponse {
                                data: Some(
                                    crate::grpc::proto::process_stream_response::Data::VideoChunk(
                                        video,
                                    ),
                                ),
                            })
                        }
                        StreamChunk::Complete {
                            input_tokens,
                            output_tokens,
                            finish_reason,
                        } => {
                            // Record metrics with tenant_id for billing
                            record_tokens(
                                &tenant_id_clone,
                                &model_clone,
                                input_tokens,
                                output_tokens,
                            );
                            record_genai_request(
                                &tenant_id_clone,
                                output_format_str(output_format_clone),
                                &model_clone,
                                finish_reason_str(finish_reason),
                            );

                            let stream_duration = stream_start.elapsed();
                            dec_grpc_in_flight("ProcessStream");
                            record_grpc_request(
                                "ProcessStream",
                                "OK",
                                stream_duration.as_secs_f64(),
                            );

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
                        dec_grpc_in_flight("ProcessStream");
                        record_grpc_request(
                            "ProcessStream",
                            "INTERNAL",
                            stream_start.elapsed().as_secs_f64(),
                        );
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
                    dec_grpc_in_flight("ProcessStream");
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::ProcessStreamStream))
    }

    #[tracing::instrument(skip(self, request), fields(tenant_id, user_id, session_id))]
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let start = Instant::now();
        let method = "CreateSession";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_SESSION_CREATE)
                .await?;
        }

        let req = request.into_inner();

        // Extract metadata
        let metadata = req.metadata.ok_or_else(|| {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!("CreateSession called without metadata");
            Status::invalid_argument("metadata is required")
        })?;

        let span = tracing::Span::current();
        span.record("tenant_id", &metadata.tenant_id);
        span.record("user_id", &metadata.user_id);

        // Convert proto documents to session documents
        let documents: Vec<SessionDocument> = req
            .documents
            .iter()
            .map(|d| {
                SessionDocument::new(
                    d.document_id.clone(),
                    d.signed_url.clone(),
                    d.mime_type.clone(),
                    d.text_content.clone(),
                )
            })
            .collect();

        // Create session
        let session = Session::new(
            metadata.tenant_id.clone(),
            metadata.user_id.clone(),
            req.title,
            req.system_prompt,
            documents,
        );

        span.record("session_id", &session.session_id);

        tracing::info!("Creating session");

        // Insert into database
        if let Err(e) = self.state.db.insert_session(&session).await {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
            tracing::error!(error = %e, "Failed to create session");
            return Err(Status::internal(format!("Failed to create session: {}", e)));
        }

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::info!(
            duration_ms = duration.as_millis(),
            "Session created successfully"
        );

        Ok(Response::new(CreateSessionResponse {
            session: Some(session_to_proto(&session)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(session_id))]
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        let start = Instant::now();
        let method = "GetSession";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_SESSION_READ)
                .await?;
        }

        let req = request.into_inner();

        if req.session_id.is_empty() {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!("GetSession called with empty session_id");
            return Err(Status::invalid_argument("session_id is required"));
        }

        let span = tracing::Span::current();
        span.record("session_id", &req.session_id);

        tracing::info!(include_messages = req.include_messages, "Getting session");

        let session = match self.state.db.find_session(&req.session_id).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "NOT_FOUND", start.elapsed().as_secs_f64());
                tracing::warn!("Session not found");
                return Err(Status::not_found(format!(
                    "Session not found: {}",
                    req.session_id
                )));
            }
            Err(e) => {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
                tracing::error!(error = %e, "Failed to get session");
                return Err(Status::internal(format!("Failed to get session: {}", e)));
            }
        };

        // Convert messages if requested
        let messages = if req.include_messages {
            session
                .messages
                .iter()
                .map(session_message_to_proto)
                .collect()
        } else {
            vec![]
        };

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::info!(
            duration_ms = duration.as_millis(),
            message_count = messages.len(),
            "Session retrieved"
        );

        Ok(Response::new(GetSessionResponse {
            session: Some(session_to_proto(&session)),
            messages,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(session_id))]
    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<DeleteSessionResponse>, Status> {
        let start = Instant::now();
        let method = "DeleteSession";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_SESSION_DELETE)
                .await?;
        }

        let req = request.into_inner();

        if req.session_id.is_empty() {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!("DeleteSession called with empty session_id");
            return Err(Status::invalid_argument("session_id is required"));
        }

        let span = tracing::Span::current();
        span.record("session_id", &req.session_id);

        tracing::info!("Deleting session");

        let success = match self.state.db.delete_session(&req.session_id).await {
            Ok(deleted) => deleted,
            Err(e) => {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
                tracing::error!(error = %e, "Failed to delete session");
                return Err(Status::internal(format!("Failed to delete session: {}", e)));
            }
        };

        if !success {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "NOT_FOUND", start.elapsed().as_secs_f64());
            tracing::warn!("Session not found for deletion");
            return Err(Status::not_found(format!(
                "Session not found: {}",
                req.session_id
            )));
        }

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::info!(duration_ms = duration.as_millis(), "Session deleted");

        Ok(Response::new(DeleteSessionResponse { success: true }))
    }

    #[tracing::instrument(skip(self, request), fields(tenant_id, user_id))]
    async fn get_usage(
        &self,
        request: Request<GetUsageRequest>,
    ) -> Result<Response<GetUsageResponse>, Status> {
        let start = Instant::now();
        let method = "GetUsage";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_USAGE_READ)
                .await?;
        }

        let req = request.into_inner();

        let span = tracing::Span::current();
        if let Some(ref tid) = req.tenant_id {
            span.record("tenant_id", tid);
        }
        if let Some(ref uid) = req.user_id {
            span.record("user_id", uid);
        }

        // Convert timestamps
        let start_time = req
            .start_time
            .as_ref()
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
            .ok_or_else(|| {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
                tracing::warn!("Invalid or missing start_time");
                Status::invalid_argument("start_time is required")
            })?;

        let end_time = req
            .end_time
            .as_ref()
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
            .ok_or_else(|| {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
                tracing::warn!("Invalid or missing end_time");
                Status::invalid_argument("end_time is required")
            })?;

        tracing::info!(
            start_time = %start_time,
            end_time = %end_time,
            "Getting usage statistics"
        );

        // Query usage records
        let records = match self
            .state
            .db
            .get_usage(
                req.tenant_id.as_deref(),
                req.user_id.as_deref(),
                start_time,
                end_time,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                dec_grpc_in_flight(method);
                record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
                tracing::error!(error = %e, "Failed to get usage");
                return Err(Status::internal(format!("Failed to get usage: {}", e)));
            }
        };

        // Aggregate statistics
        let stats = crate::models::UsageStats::from_records(&records);

        // Convert to proto response
        let by_model: Vec<crate::grpc::proto::ModelUsage> = stats
            .by_model
            .values()
            .map(|m| crate::grpc::proto::ModelUsage {
                model: m.model.clone(),
                tokens: m.tokens,
                requests: m.requests,
            })
            .collect();

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::info!(
            duration_ms = duration.as_millis(),
            record_count = records.len(),
            total_requests = stats.total_requests,
            "Usage statistics retrieved"
        );

        Ok(Response::new(GetUsageResponse {
            total_input_tokens: stats.total_input_tokens,
            total_output_tokens: stats.total_output_tokens,
            total_tokens: stats.total_tokens,
            total_requests: stats.total_requests,
            by_model,
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn list_models(
        &self,
        request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
        let start = Instant::now();
        let method = "ListModels";
        inc_grpc_in_flight(method);

        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::GENAI_MODELS_READ)
                .await?;
        }

        tracing::debug!("Listing available models");

        // Return configured models
        let models = vec![
            ModelInfo {
                id: self.state.config.models.text_model.clone(),
                name: "Text Model".to_string(),
                provider: "google".to_string(),
                supports_vision: true,
                supports_audio_output: false,
                supports_video_output: false,
                supports_streaming: true,
                context_window: 1_000_000,
            },
            ModelInfo {
                id: self.state.config.models.audio_model.clone(),
                name: "Audio Model".to_string(),
                provider: "google".to_string(),
                supports_vision: true,
                supports_audio_output: true,
                supports_video_output: false,
                supports_streaming: true,
                context_window: 1_000_000,
            },
            ModelInfo {
                id: self.state.config.models.video_model.clone(),
                name: "Video Model".to_string(),
                provider: "google".to_string(),
                supports_vision: false,
                supports_audio_output: false,
                supports_video_output: true,
                supports_streaming: false,
                context_window: 0,
            },
        ];

        let duration = start.elapsed();
        dec_grpc_in_flight(method);
        record_grpc_request(method, "OK", duration.as_secs_f64());

        tracing::debug!(
            duration_ms = duration.as_millis(),
            model_count = models.len(),
            "Models listed"
        );

        Ok(Response::new(ListModelsResponse { models }))
    }
}
