//! Gemini AI provider implementation.
//!
//! Implements text generation using Google's Gemini API.
//! Supports both streaming and non-streaming responses.

use super::{
    AudioProvider, DocumentContext, FinishReason, GenerationParams, ProviderError,
    ProviderResponse, ProviderStream, StreamChunk, TextProvider,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Gemini API base URL.
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Gemini provider configuration.
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

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
    fn api_url(&self, method: &str) -> String {
        format!(
            "{}/models/{}:{}?key={}",
            GEMINI_API_BASE, self.config.model, method, self.config.api_key
        )
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
    async fn generate(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
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

        let url = self.api_url("generateContent");

        tracing::debug!(
            model = %self.config.model,
            prompt_len = prompt.len(),
            doc_count = documents.len(),
            "Sending request to Gemini API"
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited);
            }

            return Err(ProviderError::ApiError(format!(
                "Gemini API error {}: {}",
                status, error_text
            )));
        }

        let api_response: GenerateContentResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ApiError(format!("Failed to parse response: {}", e)))?;

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
            return Err(ProviderError::ContentFiltered);
        }

        Ok(ProviderResponse {
            text,
            audio: None,
            video: None,
            input_tokens: usage.prompt_token_count.unwrap_or(0),
            output_tokens: usage.candidates_token_count.unwrap_or(0),
            finish_reason,
        })
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError> {
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

        let url = self.api_url("streamGenerateContent");
        let url = format!("{}&alt=sse", url);

        tracing::debug!(
            model = %self.config.model,
            prompt_len = prompt.len(),
            doc_count = documents.len(),
            "Starting streaming request to Gemini API"
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited);
            }

            return Err(ProviderError::ApiError(format!(
                "Gemini API error {}: {}",
                status, error_text
            )));
        }

        // Create channel for streaming
        let (tx, rx) = mpsc::channel(32);

        // Spawn task to process SSE stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut total_input_tokens = 0i32;
            let mut total_output_tokens = 0i32;
            let mut last_finish_reason = FinishReason::Complete;

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
                        let _ = tx
                            .send(Err(ProviderError::NetworkError(e.to_string())))
                            .await;
                        return;
                    }
                }
            }

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

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.config.api_key.is_empty() {
            return Err(ProviderError::NotConfigured(
                "Gemini API key not configured".to_string(),
            ));
        }

        // Try to list models to verify API key works
        let url = format!("{}/models?key={}", GEMINI_API_BASE, self.config.api_key);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::ApiError(format!(
                "Health check failed: {}",
                response.status()
            )))
        }
    }
}

/// Gemini audio provider (TTS) - placeholder for future implementation.
#[allow(dead_code)]
pub struct GeminiAudioProvider {
    config: GeminiConfig,
    client: Client,
}

impl GeminiAudioProvider {
    pub fn new(config: GeminiConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }
}

#[async_trait]
impl AudioProvider for GeminiAudioProvider {
    async fn generate(
        &self,
        _prompt: &str,
        _params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
        Err(ProviderError::NotConfigured(
            "Audio generation not yet implemented".to_string(),
        ))
    }

    async fn generate_stream(
        &self,
        _prompt: &str,
        _params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError> {
        Err(ProviderError::NotConfigured(
            "Audio streaming not yet implemented".to_string(),
        ))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.config.api_key.is_empty() {
            Err(ProviderError::NotConfigured(
                "Gemini API key not configured".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Gemini API Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<ContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ContentPart {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Content,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<i32>,
    candidates_token_count: Option<i32>,
    #[allow(dead_code)]
    total_token_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SafetyRating {
    category: String,
    probability: String,
}
