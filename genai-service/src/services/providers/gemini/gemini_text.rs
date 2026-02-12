//! Gemini text generation provider.
//!
//! Implements the `TextProvider` trait for Google's Gemini API,
//! supporting both streaming and non-streaming text generation.

use super::{
    Content, ContentPart, GeminiConfig, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, GEMINI_API_BASE, PROVIDER_NAME,
};
use crate::services::providers::{
    DocumentContext, FinishReason, GenerationParams, ProviderError, ProviderResponse,
    ProviderStream, StreamChunk, TextProvider,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Gemini text provider.
pub struct GeminiTextProvider {
    config: GeminiConfig,
    client: Client,
}

impl GeminiTextProvider {
    pub fn new(config: GeminiConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Build the API URL for the given model and method.
    fn api_url(&self, model: &str, method: &str) -> String {
        format!(
            "{}/models/{}:{}?key={}",
            GEMINI_API_BASE, model, method, self.config.api_key
        )
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

    /// Convert documents to Gemini content parts.
    fn documents_to_parts(&self, documents: &[DocumentContext]) -> Vec<ContentPart> {
        documents
            .iter()
            .map(|doc| {
                // If we have pre-extracted text, use it
                if let Some(text) = &doc.text_content {
                    return ContentPart::Text {
                        text: format!("[Document {}]: {}", doc.document_id, text),
                    };
                }

                // Otherwise, include as inline data if it's an image
                if doc.mime_type.starts_with("image/") {
                    // For images, we'd need to fetch and base64 encode
                    // For now, just note that the document exists
                    return ContentPart::Text {
                        text: format!(
                            "[Document {} - {} - URL: {}]",
                            doc.document_id, doc.mime_type, doc.url
                        ),
                    };
                }

                // For other types, note the document
                ContentPart::Text {
                    text: format!(
                        "[Document {} - {} available at {}]",
                        doc.document_id, doc.mime_type, doc.url
                    ),
                }
            })
            .collect()
    }

    /// Build generation config from parameters.
    fn build_generation_config(&self, params: &GenerationParams) -> GenerationConfig {
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
            response_schema: params
                .output_schema
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
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

        // Build content parts
        let mut parts: Vec<ContentPart> = self.documents_to_parts(documents);
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

        let url = self.api_url(&model, "generateContent");

        tracing::debug!("Sending request to Gemini API");

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                let err = ProviderError::NetworkError(e.to_string());
                tracing::error!(error = %e, "Network error calling Gemini API");
                err
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                tracing::warn!("Rate limited by Gemini API");
                return Err(ProviderError::RateLimited);
            }

            let err =
                ProviderError::ApiError(format!("Gemini API error {}: {}", status, error_text));
            tracing::error!(
                status = %status,
                error = %error_text,
                "Gemini API returned error"
            );
            return Err(err);
        }

        let api_response: GenerateContentResponse = response.json().await.map_err(|e| {
            let err = ProviderError::ApiError(format!("Failed to parse response: {}", e));
            tracing::error!(error = %e, "Failed to parse Gemini API response");
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
            audio: None,
            video: None,
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

        // Build content parts
        let mut parts: Vec<ContentPart> = self.documents_to_parts(documents);
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

        let url = self.api_url(&model, "streamGenerateContent");
        let url = format!("{}&alt=sse", url);

        tracing::debug!("Starting streaming request to Gemini API");

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                let err = ProviderError::NetworkError(e.to_string());
                tracing::error!(error = %e, "Network error starting Gemini stream");
                err
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                tracing::warn!("Rate limited by Gemini API");
                return Err(ProviderError::RateLimited);
            }

            let err =
                ProviderError::ApiError(format!("Gemini API error {}: {}", status, error_text));
            tracing::error!(
                status = %status,
                error = %error_text,
                "Gemini API returned error"
            );
            return Err(err);
        }

        let connect_duration = start.elapsed();
        tracing::debug!(
            duration_ms = connect_duration.as_millis(),
            "Gemini stream connection established"
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
                                if let Ok(response) =
                                    serde_json::from_str::<GenerateContentResponse>(data)
                                {
                                    // Update token counts
                                    if let Some(usage) = &response.usage_metadata {
                                        total_input_tokens = usage.prompt_token_count.unwrap_or(0);
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
                                                let _ = tx
                                                    .send(Ok(StreamChunk::Text(text.clone())))
                                                    .await;
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
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Error in Gemini stream");
                        let _ = tx
                            .send(Err(ProviderError::NetworkError(e.to_string())))
                            .await;
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
                "Gemini stream completed"
            );

            // Send completion
            let _ = tx
                .send(Ok(StreamChunk::Complete {
                    input_tokens: total_input_tokens,
                    output_tokens: total_output_tokens,
                    finish_reason: last_finish_reason,
                }))
                .await;
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream) as ProviderStream)
    }

    #[tracing::instrument(skip(self), fields(provider = PROVIDER_NAME))]
    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.config.api_key.is_empty() {
            tracing::warn!("Gemini API key not configured");
            return Err(ProviderError::NotConfigured(
                "Gemini API key not configured".to_string(),
            ));
        }

        let start = Instant::now();

        // Try to list models to verify API key works
        let url = format!("{}/models?key={}", GEMINI_API_BASE, self.config.api_key);

        let response = self.client.get(&url).send().await.map_err(|e| {
            tracing::error!(error = %e, "Health check network error");
            ProviderError::NetworkError(e.to_string())
        })?;

        let duration = start.elapsed();

        if response.status().is_success() {
            tracing::debug!(
                duration_ms = duration.as_millis(),
                "Gemini health check passed"
            );
            Ok(())
        } else {
            let status = response.status();
            tracing::error!(
                status = %status,
                duration_ms = duration.as_millis(),
                "Gemini health check failed"
            );
            Err(ProviderError::ApiError(format!(
                "Health check failed: {}",
                status
            )))
        }
    }
}
