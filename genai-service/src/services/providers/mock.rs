//! Mock provider implementations for testing.

use super::{
    AudioProvider, DocumentContext, FinishReason, GenerationParams, ProviderError,
    ProviderResponse, ProviderStream, StreamChunk, TextProvider, VideoProvider,
};
use async_trait::async_trait;

/// Mock text provider for testing.
pub struct MockTextProvider {
    enabled: bool,
}

impl MockTextProvider {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait]
impl TextProvider for MockTextProvider {
    async fn generate(
        &self,
        prompt: &str,
        _documents: &[DocumentContext],
        _params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured(
                "Mock text provider not enabled".to_string(),
            ));
        }

        // Simulate some processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(ProviderResponse {
            text: Some(format!("Mock response for: {}", prompt)),
            audio: None,
            video: None,
            input_tokens: prompt.len() as i32 / 4,
            output_tokens: 10,
            finish_reason: FinishReason::Complete,
        })
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _documents: &[DocumentContext],
        _params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured(
                "Mock text provider not enabled".to_string(),
            ));
        }

        let input_tokens = prompt.len() as i32 / 4;
        let prompt_text = format!(" {}", prompt);

        let chunks: Vec<Result<StreamChunk, ProviderError>> = vec![
            Ok(StreamChunk::Text("Mock".to_string())),
            Ok(StreamChunk::Text(" streaming".to_string())),
            Ok(StreamChunk::Text(" response".to_string())),
            Ok(StreamChunk::Text(" for:".to_string())),
            Ok(StreamChunk::Text(prompt_text)),
            Ok(StreamChunk::Complete {
                input_tokens,
                output_tokens: 5,
                finish_reason: FinishReason::Complete,
            }),
        ];

        Ok(Box::pin(tokio_stream::iter(chunks)))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.enabled {
            Ok(())
        } else {
            Err(ProviderError::NotConfigured(
                "Mock text provider not enabled".to_string(),
            ))
        }
    }
}

/// Mock audio provider for testing.
pub struct MockAudioProvider {
    enabled: bool,
}

impl MockAudioProvider {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait]
impl AudioProvider for MockAudioProvider {
    async fn generate(
        &self,
        prompt: &str,
        _params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured(
                "Mock audio provider not enabled".to_string(),
            ));
        }

        // Return mock audio bytes (empty for now)
        Ok(ProviderResponse {
            text: None,
            audio: Some(vec![0u8; 1024]), // Placeholder audio bytes
            video: None,
            input_tokens: prompt.len() as i32 / 4,
            output_tokens: 100,
            finish_reason: FinishReason::Complete,
        })
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _params: &GenerationParams,
    ) -> Result<ProviderStream, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured(
                "Mock audio provider not enabled".to_string(),
            ));
        }

        let input_tokens = prompt.len() as i32 / 4;
        let chunks: Vec<Result<StreamChunk, ProviderError>> = vec![
            Ok(StreamChunk::Audio(vec![0u8; 512])),
            Ok(StreamChunk::Audio(vec![0u8; 512])),
            Ok(StreamChunk::Complete {
                input_tokens,
                output_tokens: 100,
                finish_reason: FinishReason::Complete,
            }),
        ];

        Ok(Box::pin(tokio_stream::iter(chunks)))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.enabled {
            Ok(())
        } else {
            Err(ProviderError::NotConfigured(
                "Mock audio provider not enabled".to_string(),
            ))
        }
    }
}

/// Mock video provider for testing.
pub struct MockVideoProvider {
    enabled: bool,
}

impl MockVideoProvider {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait]
impl VideoProvider for MockVideoProvider {
    async fn generate(
        &self,
        prompt: &str,
        _params: &GenerationParams,
    ) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured(
                "Mock video provider not enabled".to_string(),
            ));
        }

        // Return mock video bytes (empty for now)
        Ok(ProviderResponse {
            text: None,
            audio: None,
            video: Some(vec![0u8; 4096]), // Placeholder video bytes
            input_tokens: prompt.len() as i32 / 4,
            output_tokens: 1000,
            finish_reason: FinishReason::Complete,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.enabled {
            Ok(())
        } else {
            Err(ProviderError::NotConfigured(
                "Mock video provider not enabled".to_string(),
            ))
        }
    }
}
