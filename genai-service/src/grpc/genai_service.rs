use crate::config::OutputFormat;
use crate::grpc::proto::{
    gen_ai_service_server::GenAiService, CreateSessionRequest, CreateSessionResponse,
    DeleteSessionRequest, DeleteSessionResponse, FinishReason, GetSessionRequest,
    GetSessionResponse, GetUsageRequest, GetUsageResponse, ListModelsRequest, ListModelsResponse,
    ModelInfo, OutputFormat as ProtoOutputFormat, ProcessRequest, ProcessResponse,
    ProcessStreamRequest, ProcessStreamResponse, Session as ProtoSession,
    SessionMessage as ProtoSessionMessage, StreamComplete, TokenUsage,
};
use crate::models::{Session, SessionDocument, SessionMessage, UsageRecord};
use crate::services::providers::{
    DocumentContext, FinishReason as ProviderFinishReason, GenerationParams, ProviderError,
    StreamChunk,
};
use crate::startup::AppState;
use chrono::Utc;
use futures::{Stream, StreamExt};
use prost_types::Timestamp;
use std::pin::Pin;
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

/// Convert provider finish reason to proto enum.
fn finish_reason_to_proto(reason: ProviderFinishReason) -> i32 {
    match reason {
        ProviderFinishReason::Complete => FinishReason::Complete as i32,
        ProviderFinishReason::Length => FinishReason::Length as i32,
        ProviderFinishReason::ContentFilter => FinishReason::ContentFilter as i32,
        ProviderFinishReason::Error => FinishReason::Error as i32,
    }
}

/// Convert provider error to gRPC status.
fn provider_error_to_status(error: ProviderError) -> Status {
    match error {
        ProviderError::NotConfigured(msg) => Status::failed_precondition(msg),
        ProviderError::ApiError(msg) => Status::internal(format!("Provider API error: {}", msg)),
        ProviderError::InvalidRequest(msg) => Status::invalid_argument(msg),
        ProviderError::RateLimited => Status::resource_exhausted("Rate limited by AI provider"),
        ProviderError::ContentFiltered => {
            Status::invalid_argument("Content was filtered by AI provider safety settings")
        }
        ProviderError::NetworkError(msg) => Status::unavailable(format!("Network error: {}", msg)),
    }
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

    #[tracing::instrument(skip(self, request), fields(prompt_len, output_format))]
    async fn process(
        &self,
        request: Request<ProcessRequest>,
    ) -> Result<Response<ProcessResponse>, Status> {
        let req = request.into_inner();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Validate request
        if req.prompt.is_empty() {
            return Err(Status::invalid_argument("Prompt is required"));
        }

        let output_format = proto_to_output_format(req.output_format);

        // For structured JSON, schema is required and must be valid JSON
        if output_format == OutputFormat::StructuredJson {
            match &req.output_schema {
                None => {
                    return Err(Status::invalid_argument(
                        "output_schema is required for STRUCTURED_JSON output format",
                    ));
                }
                Some(schema) => {
                    // Validate that the schema is valid JSON
                    if serde_json::from_str::<serde_json::Value>(schema).is_err() {
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

        tracing::Span::current().record("prompt_len", req.prompt.len());
        tracing::Span::current().record("output_format", format!("{:?}", output_format).as_str());

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

        tracing::info!(
            request_id = %request_id,
            model = %model,
            doc_count = enriched_documents.len(),
            "Processing request with Gemini provider"
        );

        // Call the provider
        let provider_response = self
            .state
            .text_provider
            .generate(&req.prompt, &enriched_documents, &params)
            .await
            .map_err(provider_error_to_status)?;

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

        tracing::info!(
            request_id = %request_id,
            input_tokens = provider_response.input_tokens,
            output_tokens = provider_response.output_tokens,
            "Request completed successfully"
        );

        // Record usage
        if let Some(ref metadata) = req.metadata {
            let output_format_str = match output_format {
                OutputFormat::Text => "text",
                OutputFormat::StructuredJson => "json",
                OutputFormat::Audio => "audio",
                OutputFormat::Video => "video",
            };

            let usage_record = UsageRecord::new(
                request_id.clone(),
                req.session_id.clone(),
                metadata.tenant_id.clone(),
                metadata.user_id.clone(),
                model.clone(),
                provider_response.input_tokens,
                provider_response.output_tokens,
                output_format_str.to_string(),
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

    #[tracing::instrument(skip(self, request))]
    async fn process_stream(
        &self,
        request: Request<ProcessStreamRequest>,
    ) -> Result<Response<Self::ProcessStreamStream>, Status> {
        let req = request.into_inner();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Validate request
        if req.prompt.is_empty() {
            return Err(Status::invalid_argument("Prompt is required"));
        }

        let output_format = proto_to_output_format(req.output_format);

        // For structured JSON, schema is required and must be valid JSON
        if output_format == OutputFormat::StructuredJson {
            match &req.output_schema {
                None => {
                    return Err(Status::invalid_argument(
                        "output_schema is required for STRUCTURED_JSON output format",
                    ));
                }
                Some(schema) => {
                    // Validate that the schema is valid JSON
                    if serde_json::from_str::<serde_json::Value>(schema).is_err() {
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

        tracing::info!(
            request_id = %request_id,
            model = %model,
            doc_count = enriched_documents.len(),
            "Starting streaming request with Gemini provider"
        );

        // Get streaming response from provider
        let mut provider_stream = self
            .state
            .text_provider
            .generate_stream(&req.prompt, &enriched_documents, &params)
            .await
            .map_err(provider_error_to_status)?;

        // Create channel for streaming responses
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Clone values for the spawned task
        let session_id = req.session_id.clone();
        let model_clone = model.clone();
        let request_id_clone = request_id.clone();

        // Spawn task to transform provider stream to gRPC stream
        tokio::spawn(async move {
            while let Some(chunk_result) = provider_stream.next().await {
                let grpc_response = match chunk_result {
                    Ok(chunk) => match chunk {
                        StreamChunk::Text(text) => Ok(ProcessStreamResponse {
                            data: Some(
                                crate::grpc::proto::process_stream_response::Data::TextChunk(text),
                            ),
                        }),
                        StreamChunk::Audio(audio) => Ok(ProcessStreamResponse {
                            data: Some(
                                crate::grpc::proto::process_stream_response::Data::AudioChunk(
                                    audio,
                                ),
                            ),
                        }),
                        StreamChunk::Video(video) => Ok(ProcessStreamResponse {
                            data: Some(
                                crate::grpc::proto::process_stream_response::Data::VideoChunk(
                                    video,
                                ),
                            ),
                        }),
                        StreamChunk::Complete {
                            input_tokens,
                            output_tokens,
                            finish_reason,
                        } => {
                            let complete = StreamComplete {
                                output_format: output_format_to_proto(output_format),
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

                            tracing::info!(
                                request_id = %request_id_clone,
                                input_tokens = input_tokens,
                                output_tokens = output_tokens,
                                "Streaming request completed"
                            );

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
                    // Receiver dropped, stop processing
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::ProcessStreamStream))
    }

    #[tracing::instrument(skip(self, request))]
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let req = request.into_inner();

        // Extract metadata
        let metadata = req
            .metadata
            .ok_or_else(|| Status::invalid_argument("metadata is required"))?;

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

        tracing::info!(
            session_id = %session.session_id,
            tenant_id = %metadata.tenant_id,
            user_id = %metadata.user_id,
            "Creating session"
        );

        // Insert into database
        self.state.db.insert_session(&session).await.map_err(|e| {
            tracing::error!("Failed to create session: {}", e);
            Status::internal(format!("Failed to create session: {}", e))
        })?;

        tracing::info!(session_id = %session.session_id, "Session created successfully");

        Ok(Response::new(CreateSessionResponse {
            session: Some(session_to_proto(&session)),
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        let req = request.into_inner();

        if req.session_id.is_empty() {
            return Err(Status::invalid_argument("session_id is required"));
        }

        tracing::info!(
            session_id = %req.session_id,
            include_messages = req.include_messages,
            "Getting session"
        );

        let session = self
            .state
            .db
            .find_session(&req.session_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get session: {}", e);
                Status::internal(format!("Failed to get session: {}", e))
            })?
            .ok_or_else(|| Status::not_found(format!("Session not found: {}", req.session_id)))?;

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

        Ok(Response::new(GetSessionResponse {
            session: Some(session_to_proto(&session)),
            messages,
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<DeleteSessionResponse>, Status> {
        let req = request.into_inner();

        if req.session_id.is_empty() {
            return Err(Status::invalid_argument("session_id is required"));
        }

        tracing::info!(session_id = %req.session_id, "Deleting session");

        let success = self
            .state
            .db
            .delete_session(&req.session_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete session: {}", e);
                Status::internal(format!("Failed to delete session: {}", e))
            })?;

        if !success {
            return Err(Status::not_found(format!(
                "Session not found: {}",
                req.session_id
            )));
        }

        tracing::info!(session_id = %req.session_id, "Session deleted successfully");

        Ok(Response::new(DeleteSessionResponse { success: true }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_usage(
        &self,
        request: Request<GetUsageRequest>,
    ) -> Result<Response<GetUsageResponse>, Status> {
        let req = request.into_inner();

        // Convert timestamps
        let start_time = req
            .start_time
            .as_ref()
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
            .ok_or_else(|| Status::invalid_argument("start_time is required"))?;

        let end_time = req
            .end_time
            .as_ref()
            .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
            .ok_or_else(|| Status::invalid_argument("end_time is required"))?;

        tracing::info!(
            start_time = %start_time,
            end_time = %end_time,
            tenant_id = ?req.tenant_id,
            user_id = ?req.user_id,
            "Getting usage"
        );

        // Query usage records
        let records = self
            .state
            .db
            .get_usage(
                req.tenant_id.as_deref(),
                req.user_id.as_deref(),
                start_time,
                end_time,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to get usage: {}", e);
                Status::internal(format!("Failed to get usage: {}", e))
            })?;

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

        Ok(Response::new(GetUsageResponse {
            total_input_tokens: stats.total_input_tokens,
            total_output_tokens: stats.total_output_tokens,
            total_tokens: stats.total_tokens,
            total_requests: stats.total_requests,
            by_model,
        }))
    }

    #[tracing::instrument(skip(self, _request))]
    async fn list_models(
        &self,
        _request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
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

        Ok(Response::new(ListModelsResponse { models }))
    }
}
