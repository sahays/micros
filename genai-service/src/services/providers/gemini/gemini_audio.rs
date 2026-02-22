//! Gemini audio generation provider.
//!
//! Implements the `AudioProvider` trait for Google's Gemini TTS API.
//! Currently a placeholder for future implementation.

use super::GeminiConfig;
use crate::services::providers::{
    AudioProvider, GenerationParams, ProviderError, ProviderResponse, ProviderStream,
};
use async_trait::async_trait;
use reqwest::Client;

/// Gemini audio provider (TTS) - placeholder for future implementation.
pub struct GeminiAudioProvider {
    config: GeminiConfig,
    client: Client,
}

impl GeminiAudioProvider {
    pub fn new(config: GeminiConfig) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
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
        self.config
            .get_access_token(&self.client)
            .await
            .map_err(|e| {
                ProviderError::NotConfigured(format!(
                    "Service account authentication failed: {}",
                    e
                ))
            })?;
        Ok(())
    }
}
