use crate::config::AuthServiceSettings;
use anyhow::Result;
use reqwest::Client;
use service_core::observability::TracedClientExt;

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

    pub fn base_url(&self) -> &str {
        &self.settings.url
    }

    pub fn public_url(&self) -> &str {
        &self.settings.public_url
    }

    /// Send a POST request with trace context propagation.
    ///
    /// Trace context (traceparent/tracestate) is automatically injected
    /// for distributed tracing across services.
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.settings.url, path);

        let response = self
            .client
            .traced_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send POST request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }

    /// Send a GET request with auth token and trace context propagation.
    pub async fn get_with_auth(&self, path: &str, access_token: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.settings.url, path);

        let response = self
            .client
            .traced_get(&url)
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
