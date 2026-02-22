//! AI provider abstractions and implementations.
//!
//! This module provides a trait-based abstraction for AI providers,
//! allowing easy swapping between different backends (Gemini, mock).

pub mod gemini;
pub mod mock;

use async_trait::async_trait;
use std::pin::Pin;
use thiserror::Error;
use tokio_stream::Stream;

/// Error type for provider operations.
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Content filtered")]
    ContentFiltered,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timed out after {0}s")]
    Timeout(u64),
}

/// Result of a provider response.
pub struct ProviderResponse {
    /// Text content (for TEXT/STRUCTURED_JSON).
    pub text: Option<String>,

    /// Input tokens consumed.
    pub input_tokens: i32,

    /// Output tokens generated.
    pub output_tokens: i32,

    /// Finish reason.
    pub finish_reason: FinishReason,
}

/// Reason why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Complete,
    Length,
    ContentFilter,
    Error,
}

/// Stream chunk for streaming responses.
pub enum StreamChunk {
    /// Text chunk.
    Text(String),

    /// Final completion with usage stats.
    Complete {
        input_tokens: i32,
        output_tokens: i32,
        finish_reason: FinishReason,
    },
}

/// Type alias for provider streams.
pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>;

/// Generation parameters for AI requests.
#[derive(Debug, Clone, Default)]
pub struct GenerationParams {
    /// Temperature (0.0 - 2.0).
    pub temperature: Option<f32>,

    /// Top-p sampling.
    pub top_p: Option<f32>,

    /// Maximum output tokens.
    pub max_tokens: Option<i32>,

    /// Stop sequences.
    pub stop_sequences: Vec<String>,

    /// JSON schema for structured output.
    pub output_schema: Option<String>,

    /// Optional model override. When set, overrides the provider's default model.
    pub model: Option<String>,
}

/// Document context for AI requests.
#[derive(Debug, Clone)]
pub struct DocumentContext {
    /// Document ID.
    pub document_id: String,

    /// MIME type.
    pub mime_type: String,

    /// Pre-extracted text content.
    pub text_content: Option<String>,
}

/// Trait for text/JSON generation providers (e.g., Gemini).
#[async_trait]
pub trait TextProvider: Send + Sync {
    /// Generate text response.
    async fn generate(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError>;

    /// Generate streaming text response.
    async fn generate_stream(
        &self,
        prompt: &str,
        documents: &[DocumentContext],
        params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError>;

    /// Health check.
    async fn health_check(&self) -> Result<(), ProviderError>;
}
