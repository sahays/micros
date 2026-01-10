use crate::config::DocumentServiceSettings;
use crate::utils::crypto::{compute_body_hash, create_signature};
use anyhow::Result;
use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct DocumentClient {
    client: Client,
    settings: DocumentServiceSettings,
}

impl DocumentClient {
    pub fn new(settings: DocumentServiceSettings) -> Self {
        Self {
            client: Client::new(),
            settings,
        }
    }

    /// Upload a file to document-service with user context
    ///
    /// Following rest-api-security skill guidance:
    /// - HMAC signature for service-to-service auth
    /// - Timestamp validation (60s window)
    /// - Nonce for replay prevention
    /// - X-User-ID header for user context propagation
    pub async fn upload(
        &self,
        user_id: &str,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}/documents", self.settings.url);

        // Create multipart form
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(content_type)?;

        let form = reqwest::multipart::Form::new().part("file", part);

        // For multipart, we hash empty body as the form is built by reqwest
        let body_hash = compute_body_hash(b"");

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let nonce = Uuid::new_v4().to_string();

        // Create HMAC signature following rest-api-security pattern
        let signature = create_signature(
            &self.settings.signing_secret,
            "POST",
            "/documents",
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
            .header("X-User-ID", user_id) // User context for tracing and ownership
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send upload request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }

    /// Get document metadata
    pub async fn get_document(
        &self,
        user_id: &str,
        document_id: &str,
    ) -> Result<reqwest::Response> {
        let path = format!("/documents/{}", document_id);
        let url = format!("{}{}", self.settings.url, path);
        let body_hash = compute_body_hash(b"");

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let nonce = Uuid::new_v4().to_string();

        let signature = create_signature(
            &self.settings.signing_secret,
            "GET",
            &path,
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
            .header("X-User-ID", user_id)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send GET request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }
}
