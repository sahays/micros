//! Gemini AI provider implementation via Vertex AI.
//!
//! Implements text generation using Google's Gemini models through the
//! Vertex AI endpoint, authenticated with a service account key file.

mod gemini_audio;
mod gemini_text;

pub use gemini_audio::GeminiAudioProvider;
pub use gemini_text::GeminiTextProvider;

use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Provider name for metrics.
pub(crate) const PROVIDER_NAME: &str = "gemini";

// ============================================================================
// Service Account Auth
// ============================================================================

/// Deserialized Google service account key file.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    pub token_uri: String,
    pub project_id: String,
}

/// JWT claims for Google OAuth2 token exchange.
#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Cached access token with expiry.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: i64,
}

/// Google token response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

/// Gemini provider configuration backed by Vertex AI.
#[derive(Clone)]
pub struct GeminiConfig {
    pub model: String,
    /// Region for the Vertex AI endpoint (e.g., "global", "us-central1").
    pub region: String,
    pub project_id: String,
    service_account_key: Arc<ServiceAccountKey>,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
}

impl std::fmt::Debug for GeminiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiConfig")
            .field("model", &self.model)
            .field("region", &self.region)
            .field("project_id", &self.project_id)
            .finish()
    }
}

impl GeminiConfig {
    /// Create a GeminiConfig from a service account key file path, model name, and region.
    pub fn from_service_account(
        key_path: &str,
        model: &str,
        region: &str,
    ) -> Result<Self, anyhow::Error> {
        let path = Path::new(key_path);
        let key_data = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read service account key file '{}': {}",
                key_path,
                e
            )
        })?;

        let key: ServiceAccountKey = serde_json::from_str(&key_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse service account key JSON: {}", e))?;

        let project_id = key.project_id.clone();

        Ok(Self {
            model: model.to_string(),
            region: region.to_string(),
            project_id,
            service_account_key: Arc::new(key),
            cached_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Get a valid access token, refreshing if expired or not yet cached.
    pub async fn get_access_token(&self, client: &Client) -> Result<String, anyhow::Error> {
        // Check cache first
        {
            let cache = self.cached_token.read().await;
            if let Some(ref token) = *cache {
                let now = Utc::now().timestamp();
                // 60-second safety margin before expiry
                if token.expires_at - 60 > now {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Token expired or not cached - mint a new JWT and exchange it
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            iss: self.service_account_key.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: self.service_account_key.token_uri.clone(),
            iat: now,
            exp: now + 3600,
        };

        let header = Header::new(jsonwebtoken::Algorithm::RS256);
        let encoding_key =
            EncodingKey::from_rsa_pem(self.service_account_key.private_key.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to create RSA encoding key: {}", e))?;

        let jwt_token = encode(&header, &claims, &encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT: {}", e))?;

        // Exchange JWT for access token
        let response = client
            .post(&self.service_account_key.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt_token),
            ])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Token exchange request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Token exchange failed with status {}: {}",
                status,
                body
            ));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse token response: {}", e))?;

        let cached = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at: now + token_response.expires_in,
        };

        // Update cache
        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(cached);
        }

        Ok(token_response.access_token)
    }

    /// Build a Vertex AI URL for the given model and method.
    /// Uses the configured region (e.g., "global", "us-central1").
    pub fn vertex_url(&self, model: &str, method: &str) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:{}",
            self.region, self.project_id, self.region, model, method
        )
    }
}

// ============================================================================
// Gemini API Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<ContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum ContentPart {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InlineData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SafetySetting {
    pub category: String,
    pub threshold: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerateContentResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Candidate {
    pub content: Content,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    #[allow(dead_code)]
    pub total_token_count: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct SafetyRating {
    pub category: String,
    pub probability: String,
}
