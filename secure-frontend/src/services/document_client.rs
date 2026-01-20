use crate::config::DocumentServiceSettings;
use anyhow::Result;
use reqwest::Client;
use service_core::observability::{inject_trace_context, TracedClientExt};

pub struct DocumentClient {
    client: Client,
    pub settings: DocumentServiceSettings,
}

impl DocumentClient {
    pub fn new(settings: DocumentServiceSettings) -> Self {
        Self {
            client: Client::new(),
            settings,
        }
    }

    /// Upload a file to document-service with user context.
    ///
    /// Trace context (traceparent/tracestate) is automatically injected
    /// for distributed tracing across services.
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

        // Inject trace context manually for multipart requests
        let mut headers = reqwest::header::HeaderMap::new();
        inject_trace_context(&mut headers);

        let response = self
            .client
            .post(&url)
            .headers(headers)
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

    /// Get document metadata.
    pub async fn get_document(
        &self,
        user_id: &str,
        document_id: &str,
    ) -> Result<reqwest::Response> {
        let url = format!("{}/documents/{}", self.settings.url, document_id);

        let response = self
            .client
            .traced_get(&url)
            .header("X-User-ID", user_id)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send GET request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }

    /// Download document content.
    ///
    /// Downloads the file content from document-service.
    /// Returns original or processed file depending on processing status.
    pub async fn download_document(
        &self,
        user_id: &str,
        document_id: &str,
    ) -> Result<reqwest::Response> {
        let url = format!("{}/documents/{}/content", self.settings.url, document_id);

        let response = self
            .client
            .traced_get(&url)
            .header("X-User-ID", user_id)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send download request to {}: {}", url, e);
                anyhow::anyhow!("HTTP request failed: {}", e)
            })?;

        Ok(response)
    }
}
