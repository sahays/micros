use crate::config::AuthServiceSettings;
use crate::utils::crypto::{compute_body_hash, create_signature};
use anyhow::Result;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct AuthClient {
    client: Client,
    settings: AuthServiceSettings,
}

impl AuthClient {
    pub fn new(settings: AuthServiceSettings) -> Self {
        Self {
            client: Client::new(),
            settings,
        }
    }

    pub async fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.settings.url, path);
        let body_bytes = serde_json::to_vec(&body)?;
        let body_hash = compute_body_hash(&body_bytes);

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let nonce = Uuid::new_v4().to_string();

        let signature = create_signature(
            &self.settings.signing_secret,
            "POST",
            path,
            timestamp,
            &nonce,
            &body_hash,
        );

        let response = self
            .client
            .post(&url)
            .header("X-Client-ID", &self.settings.client_id)
            .header("X-Timestamp", timestamp)
            .header("X-Nonce", nonce)
            .header("X-Signature", signature)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send POST request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }

    pub async fn get_with_auth(&self, path: &str, access_token: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.settings.url, path);
        let body_hash = compute_body_hash(b""); // Empty body for GET

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let nonce = Uuid::new_v4().to_string();

        let signature = create_signature(
            &self.settings.signing_secret,
            "GET",
            path,
            timestamp,
            &nonce,
            &body_hash,
        );

        let response = self
            .client
            .get(&url)
            .header("X-Client-ID", &self.settings.client_id)
            .header("X-Timestamp", timestamp)
            .header("X-Nonce", nonce)
            .header("X-Signature", signature)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send GET request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }
}
