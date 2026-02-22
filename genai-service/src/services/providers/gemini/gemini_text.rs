//! Gemini text generation provider via Vertex AI.
//!
//! Implements the `TextProvider` trait for Google's Gemini models,
//! supporting both streaming and non-streaming text generation
//! through the Vertex AI endpoint with service account auth.

use super::{
    Content, ContentPart, GeminiConfig, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, InlineData, PROVIDER_NAME,
};
use crate::services::document_fetcher::{DocumentFetcher, TenantContext};
use crate::services::providers::{
    DocumentContext, FinishReason, GenerationParams, ProviderError, ProviderResponse,
    ProviderStream, StreamChunk, TextProvider,
};
use async_trait::async_trait;
use base64::Engine;
use futures::StreamExt;
use reqwest::Client;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const REQUEST_TIMEOUT_SECS: u64 = 300;
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Gemini text provider.
pub struct GeminiTextProvider {
    config: GeminiConfig,
    client: Client,
    document_fetcher: DocumentFetcher,
}

impl GeminiTextProvider {
    pub fn new(
        config: GeminiConfig,
        document_fetcher: DocumentFetcher,
    ) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                ProviderError::NotConfigured(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            config,
            client,
            document_fetcher,
        })
    }

    /// Resolve the effective model: use the override from params if present,
    /// otherwise fall back to the provider's configured default.
    fn resolve_model(&self, params: &GenerationParams) -> String {
        params
            .model
            .as_deref()
            .unwrap_or(&self.config.model)
            .to_string()
    }

    /// Check if a MIME type is text-based (content can be read as UTF-8).
    fn is_text_mime_type(mime_type: &str) -> bool {
        mime_type.starts_with("text/")
            || mime_type == "application/json"
            || mime_type == "application/xml"
            || mime_type == "application/x-yaml"
    }

    /// Convert documents to Gemini content parts.
    ///
    /// Strategy by MIME type:
    /// 1. Pre-extracted text available → use it directly
    /// 2. Inline-capable (images, PDFs) → download as base64 inlineData
    /// 3. Text-based (CSV, plain text, JSON, etc.) → download and read as UTF-8 text
    /// 4. Otherwise → fail with a descriptive error
    async fn documents_to_parts(
        &self,
        documents: &[DocumentContext],
        tenant: Option<&TenantContext>,
    ) -> Result<Vec<ContentPart>, ProviderError> {
        let mut parts = Vec::with_capacity(documents.len());

        for doc in documents {
            // 1. If we have pre-extracted text, use it
            if let Some(text) = &doc.text_content {
                parts.push(ContentPart::Text {
                    text: format!("[Document {}]: {}", doc.document_id, text),
                });
                continue;
            }

            let tc = tenant.ok_or_else(|| {
                ProviderError::InvalidRequest(format!(
                    "Document {} requires tenant context for download",
                    doc.document_id
                ))
            })?;

            // 2. Inline-capable (images, PDFs) → download as base64
            if DocumentFetcher::supports_inline(&doc.mime_type) {
                let content = self
                    .document_fetcher
                    .download_content(&doc.document_id, tc)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            document_id = %doc.document_id,
                            mime_type = %doc.mime_type,
                            error = %e,
                            "Failed to download document for inline submission"
                        );
                        ProviderError::InvalidRequest(format!(
                            "Failed to download document {}: {}",
                            doc.document_id, e
                        ))
                    })?;

                let b64 = base64::engine::general_purpose::STANDARD.encode(&content.data);
                tracing::info!(
                    document_id = %doc.document_id,
                    mime_type = %content.mime_type,
                    size_bytes = content.data.len(),
                    "Downloaded document for inline submission"
                );
                parts.push(ContentPart::InlineData {
                    inline_data: InlineData {
                        mime_type: content.mime_type,
                        data: b64,
                    },
                });
                continue;
            }

            // 3. Text-based MIME types → download and read raw content as text
            if Self::is_text_mime_type(&doc.mime_type) {
                let content = self
                    .document_fetcher
                    .download_content(&doc.document_id, tc)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            document_id = %doc.document_id,
                            mime_type = %doc.mime_type,
                            error = %e,
                            "Failed to download text document"
                        );
                        ProviderError::InvalidRequest(format!(
                            "Failed to download document {}: {}",
                            doc.document_id, e
                        ))
                    })?;

                let text = String::from_utf8_lossy(&content.data).to_string();
                tracing::info!(
                    document_id = %doc.document_id,
                    mime_type = %doc.mime_type,
                    size_bytes = content.data.len(),
                    text_len = text.len(),
                    "Downloaded text document as raw content"
                );
                parts.push(ContentPart::Text {
                    text: format!("[Document {}]: {}", doc.document_id, text),
                });
                continue;
            }

            // 4. Unsupported — no way to send this to Gemini
            return Err(ProviderError::InvalidRequest(format!(
                "Document {} ({}) has no extracted text and is not inline-capable. \
                 Ensure the document exists and has been processed.",
                doc.document_id, doc.mime_type
            )));
        }

        Ok(parts)
    }

    /// Build generation config from parameters.
    fn build_generation_config(&self, params: &GenerationParams) -> GenerationConfig {
        let response_schema: Option<serde_json::Value> =
            params.output_schema.as_ref().and_then(|s| {
                match serde_json::from_str(s) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            schema = %s,
                            "Failed to parse output_schema as JSON, ignoring schema"
                        );
                        None
                    }
                }
            });

        if let Some(ref schema) = response_schema {
            tracing::info!(
                schema = %serde_json::to_string(schema).unwrap_or_default(),
                "Response schema for Vertex AI request"
            );
        }

        GenerationConfig {
            temperature: params.temperature,
            top_p: params.top_p,
            max_output_tokens: params.max_tokens,
            stop_sequences: if params.stop_sequences.is_empty() {
                None
            } else {
                Some(params.stop_sequences.clone())
            },
            response_mime_type: params
                .output_schema
                .as_ref()
                .map(|_| "application/json".to_string()),
            response_schema,
        }
    }

    /// Map a reqwest error to the appropriate ProviderError,
    /// distinguishing timeouts from other network errors.
    fn map_request_error(e: &reqwest::Error, elapsed: &std::time::Duration) -> ProviderError {
        if e.is_timeout() {
            tracing::error!(
                error = ?e,
                duration_ms = %elapsed.as_millis(),
                "Vertex AI request timed out"
            );
            ProviderError::Timeout(REQUEST_TIMEOUT_SECS)
        } else if e.is_connect() {
            tracing::error!(
                error = ?e,
                duration_ms = %elapsed.as_millis(),
                "Failed to connect to Vertex AI"
            );
            ProviderError::NetworkError(format!("Connection failed: {:#}", e))
        } else {
            tracing::error!(
                error = ?e,
                duration_ms = %elapsed.as_millis(),
                "Network error calling Vertex AI"
            );
            ProviderError::NetworkError(format!("{:#}", e))
        }
    }
}

#[async_trait]
impl TextProvider for GeminiTextProvider {
    #[tracing::instrument(
        skip(self, prompt, documents, params),
        fields(
            provider = PROVIDER_NAME,
            model,
            prompt_len = prompt.len(),
            doc_count = documents.len()
        )
    )]
    async fn generate(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
        let start = Instant::now();
        let model = self.resolve_model(params);
        tracing::Span::current().record("model", &model);

        // Get bearer token
        let access_token = self
            .config
            .get_access_token(&self.client)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get access token");
                ProviderError::ApiError(format!("Authentication failed: {}", e))
            })?;

        // Build content parts
        let mut parts: Vec<ContentPart> = self
            .documents_to_parts(documents, params.tenant_context.as_ref())
            .await?;
        parts.push(ContentPart::Text {
            text: prompt.to_string(),
        });

        let request = GenerateContentRequest {
            contents: vec![Content {
                role: Some("user".to_string()),
                parts,
            }],
            generation_config: Some(self.build_generation_config(params)),
            safety_settings: None,
        };

        let url = self.config.vertex_url(&model, "generateContent");

        tracing::info!(
            url = %url,
            has_response_schema = request.generation_config.as_ref()
                .and_then(|c| c.response_schema.as_ref()).is_some(),
            "Sending request to Vertex AI"
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&request)
            .send()
            .await
            .map_err(|e| Self::map_request_error(&e, &start.elapsed()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                tracing::warn!("Rate limited by Vertex AI");
                return Err(ProviderError::RateLimited);
            }

            let err =
                ProviderError::ApiError(format!("Vertex AI error {}: {}", status, error_text));
            tracing::error!(
                status = %status,
                error = %error_text,
                "Vertex AI returned error"
            );
            return Err(err);
        }

        let api_response: GenerateContentResponse = response.json().await.map_err(|e| {
            let err = ProviderError::ApiError(format!("Failed to parse response: {}", e));
            tracing::error!(error = %e, "Failed to parse Vertex AI response");
            err
        })?;

        // Extract text from response
        let text = api_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            });

        // Get token usage
        let usage = api_response.usage_metadata.unwrap_or_default();

        // Determine finish reason
        let finish_reason = api_response
            .candidates
            .first()
            .map(|c| match c.finish_reason.as_deref() {
                Some("STOP") => FinishReason::Complete,
                Some("MAX_TOKENS") => FinishReason::Length,
                Some("SAFETY") => FinishReason::ContentFilter,
                _ => FinishReason::Complete,
            })
            .unwrap_or(FinishReason::Complete);

        if finish_reason == FinishReason::ContentFilter {
            tracing::warn!("Content filtered by Gemini safety settings");
            return Err(ProviderError::ContentFiltered);
        }

        let duration = start.elapsed();
        let input_tokens = usage.prompt_token_count.unwrap_or(0);
        let output_tokens = usage.candidates_token_count.unwrap_or(0);

        tracing::info!(
            duration_ms = duration.as_millis(),
            input_tokens = input_tokens,
            output_tokens = output_tokens,
            finish_reason = ?finish_reason,
            "Gemini generation completed"
        );

        Ok(ProviderResponse {
            text,
            input_tokens,
            output_tokens,
            finish_reason,
        })
    }

    #[tracing::instrument(
        skip(self, prompt, documents, params),
        fields(
            provider = PROVIDER_NAME,
            model,
            prompt_len = prompt.len(),
            doc_count = documents.len()
        )
    )]
    async fn generate_stream(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError> {
        let start = Instant::now();
        let model = self.resolve_model(params);
        tracing::Span::current().record("model", &model);

        // Get bearer token
        let access_token = self
            .config
            .get_access_token(&self.client)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get access token");
                ProviderError::ApiError(format!("Authentication failed: {}", e))
            })?;

        // Build content parts
        let mut parts: Vec<ContentPart> = self
            .documents_to_parts(documents, params.tenant_context.as_ref())
            .await?;
        parts.push(ContentPart::Text {
            text: prompt.to_string(),
        });

        let request = GenerateContentRequest {
            contents: vec![Content {
                role: Some("user".to_string()),
                parts,
            }],
            generation_config: Some(self.build_generation_config(params)),
            safety_settings: None,
        };

        let url = self
            .config
            .vertex_url(&model, "streamGenerateContent?alt=sse");

        tracing::debug!("Starting streaming request to Vertex AI");

        let response = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&request)
            .send()
            .await
            .map_err(|e| Self::map_request_error(&e, &start.elapsed()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                tracing::warn!("Rate limited by Vertex AI");
                return Err(ProviderError::RateLimited);
            }

            let err =
                ProviderError::ApiError(format!("Vertex AI error {}: {}", status, error_text));
            tracing::error!(
                status = %status,
                error = %error_text,
                "Vertex AI returned error"
            );
            return Err(err);
        }

        let connect_duration = start.elapsed();
        tracing::debug!(
            duration_ms = connect_duration.as_millis(),
            "Vertex AI stream connection established"
        );

        // Create channel for streaming
        let (tx, rx) = mpsc::channel(32);

        // Spawn task to process SSE stream
        tokio::spawn(async move {
            let stream_start = Instant::now();
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut total_input_tokens = 0i32;
            let mut total_output_tokens = 0i32;
            let mut last_finish_reason = FinishReason::Complete;
            let mut chunk_count = 0u32;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Process complete SSE events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            // Parse SSE event
                            if let Some(data) = event.strip_prefix("data: ") {
                                match serde_json::from_str::<GenerateContentResponse>(data) {
                                    Ok(response) => {
                                        // Update token counts
                                        if let Some(usage) = &response.usage_metadata {
                                            total_input_tokens =
                                                usage.prompt_token_count.unwrap_or(0);
                                            total_output_tokens =
                                                usage.candidates_token_count.unwrap_or(0);
                                        }

                                        // Extract text and send
                                        if let Some(candidate) = response.candidates.first() {
                                            if let Some(ContentPart::Text { text }) =
                                                candidate.content.parts.first()
                                            {
                                                if !text.is_empty() {
                                                    chunk_count += 1;
                                                    if tx
                                                        .send(Ok(StreamChunk::Text(text.clone())))
                                                        .await
                                                        .is_err()
                                                    {
                                                        tracing::debug!(
                                                            "Client disconnected during streaming, stopping"
                                                        );
                                                        return;
                                                    }
                                                }
                                            }

                                            // Check finish reason
                                            if let Some(reason) = &candidate.finish_reason {
                                                last_finish_reason = match reason.as_str() {
                                                    "STOP" => FinishReason::Complete,
                                                    "MAX_TOKENS" => FinishReason::Length,
                                                    "SAFETY" => FinishReason::ContentFilter,
                                                    _ => FinishReason::Complete,
                                                };
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            data_len = data.len(),
                                            data_preview = %&data[..data.len().min(200)],
                                            "Failed to parse SSE event from Vertex AI"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Error in Vertex AI stream");
                        if tx
                            .send(Err(ProviderError::NetworkError(e.to_string())))
                            .await
                            .is_err()
                        {
                            tracing::debug!("Client disconnected before error could be sent");
                        }
                        return;
                    }
                }
            }

            let stream_duration = stream_start.elapsed();

            tracing::info!(
                duration_ms = stream_duration.as_millis(),
                input_tokens = total_input_tokens,
                output_tokens = total_output_tokens,
                chunk_count = chunk_count,
                finish_reason = ?last_finish_reason,
                "Vertex AI stream completed"
            );

            // Send completion
            if tx
                .send(Ok(StreamChunk::Complete {
                    input_tokens: total_input_tokens,
                    output_tokens: total_output_tokens,
                    finish_reason: last_finish_reason,
                }))
                .await
                .is_err()
            {
                tracing::debug!("Client disconnected before completion could be sent");
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream) as ProviderStream)
    }

    #[tracing::instrument(skip(self), fields(provider = PROVIDER_NAME))]
    async fn health_check(&self) -> Result<(), ProviderError> {
        let start = Instant::now();

        // Verify we can acquire an access token
        let access_token = self
            .config
            .get_access_token(&self.client)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Health check: failed to get access token");
                ProviderError::NotConfigured(format!(
                    "Service account authentication failed: {}",
                    e
                ))
            })?;

        // Verify Vertex AI is reachable by listing models
        let url = format!(
            "https://aiplatform.googleapis.com/v1/projects/{}/locations/global/publishers/google/models",
            self.config.project_id
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Health check network error");
                ProviderError::NetworkError(e.to_string())
            })?;

        let duration = start.elapsed();

        if response.status().is_success() {
            tracing::debug!(
                duration_ms = duration.as_millis(),
                "Gemini health check passed (Vertex AI)"
            );
            Ok(())
        } else {
            let status = response.status();
            tracing::error!(
                status = %status,
                duration_ms = duration.as_millis(),
                "Gemini health check failed (Vertex AI)"
            );
            Err(ProviderError::ApiError(format!(
                "Health check failed: {}",
                status
            )))
        }
    }
}
