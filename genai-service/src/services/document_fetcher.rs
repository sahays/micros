//! Document fetcher service for retrieving document content.
//!
//! This service fetches document metadata and content from the document-service
//! via gRPC, enriching document context with extracted text when available.

use crate::grpc::document_proto::{
    document_service_client::DocumentServiceClient, DownloadDocumentRequest,
    GetDocumentRequest, GetProcessingStatusRequest,
};
use crate::services::providers::DocumentContext;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tonic::transport::Channel;

/// Maximum file size for inline Gemini submission (20MB Vertex AI limit).
const MAX_INLINE_SIZE_BYTES: usize = 20 * 1024 * 1024;

/// MIME types that can be sent as Gemini inlineData.
const INLINE_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/heic",
    "image/heif",
    "application/pdf",
];

/// Downloaded file content from document-service.
pub struct DownloadedContent {
    /// Raw file bytes.
    pub data: Vec<u8>,
    /// MIME type from the download metadata.
    pub mime_type: String,
}

/// Error type for document fetching operations.
#[derive(Debug, thiserror::Error)]
pub enum DocumentFetcherError {
    #[error("Failed to connect to document service: {0}")]
    ConnectionError(String),

    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Document not ready: {0}")]
    NotReady(String),

    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),

    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[error("File too large: {0} bytes exceeds {MAX_INLINE_SIZE_BYTES} byte limit")]
    TooLarge(usize),
}

impl DocumentFetcherError {
    /// Get the error type string for metrics.
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::ConnectionError(_) => "connection",
            Self::NotFound(_) => "not_found",
            Self::NotReady(_) => "not_ready",
            Self::GrpcError(_) => "grpc",
            Self::TransportError(_) => "transport",
            Self::TooLarge(_) => "too_large",
        }
    }
}

/// Document fetcher for retrieving document content from document-service.
#[derive(Clone)]
pub struct DocumentFetcher {
    client: Arc<RwLock<Option<DocumentServiceClient<Channel>>>>,
    endpoint: String,
}

impl DocumentFetcher {
    /// Create a new document fetcher with the given endpoint.
    pub fn new(endpoint: &str) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            endpoint: endpoint.to_string(),
        }
    }

    /// Get or create the gRPC client.
    #[tracing::instrument(skip(self), fields(endpoint = %self.endpoint))]
    async fn get_client(&self) -> Result<DocumentServiceClient<Channel>, DocumentFetcherError> {
        // Check if we already have a client
        {
            let client = self.client.read().await;
            if let Some(client) = client.as_ref() {
                return Ok(client.clone());
            }
        }

        tracing::debug!("Creating new document-service gRPC client");
        let start = Instant::now();

        // Create new client
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| {
                let err = DocumentFetcherError::ConnectionError(e.to_string());
                tracing::error!(error = %e, "Invalid endpoint URL");
                err
            })?
            .connect()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to connect to document-service");
                e
            })?;

        let client = DocumentServiceClient::new(channel);
        let duration = start.elapsed();

        tracing::info!(
            duration_ms = duration.as_millis(),
            "Connected to document-service"
        );

        // Store the client
        {
            let mut guard = self.client.write().await;
            *guard = Some(client.clone());
        }

        Ok(client)
    }

    /// Enrich document context with extracted text from document-service.
    ///
    /// For each document that doesn't have pre-extracted text, this method
    /// fetches the document metadata from document-service and adds any
    /// extracted text to the context.
    #[tracing::instrument(skip(self, documents), fields(document_count = documents.len()))]
    pub async fn enrich_documents(
        &self,
        documents: &[DocumentContext],
    ) -> Result<Vec<DocumentContext>, DocumentFetcherError> {
        let start = Instant::now();
        let mut enriched = Vec::with_capacity(documents.len());
        let mut enriched_count = 0;
        let mut skipped_count = 0;
        let mut error_count = 0;

        for doc in documents {
            // If document already has text content, skip fetching
            if doc.text_content.is_some() {
                tracing::debug!(
                    document_id = %doc.document_id,
                    "Document already has text content, skipping fetch"
                );
                enriched.push(doc.clone());
                skipped_count += 1;
                continue;
            }

            // Try to fetch extracted text from document-service
            match self.fetch_document_text(&doc.document_id).await {
                Ok(Some(text)) => {
                    tracing::debug!(
                        document_id = %doc.document_id,
                        text_len = text.len(),
                        "Enriched document with extracted text"
                    );
                    enriched.push(DocumentContext {
                        document_id: doc.document_id.clone(),
                        mime_type: doc.mime_type.clone(),
                        text_content: Some(text),
                    });
                    enriched_count += 1;
                }
                Ok(None) => {
                    tracing::debug!(
                        document_id = %doc.document_id,
                        "No extracted text available for document"
                    );
                    enriched.push(doc.clone());
                    skipped_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        document_id = %doc.document_id,
                        error = %e,
                        error_type = e.error_type(),
                        "Failed to fetch document text, using original context"
                    );
                    // Continue with original document context
                    enriched.push(doc.clone());
                    error_count += 1;
                }
            }
        }

        let duration = start.elapsed();

        tracing::info!(
            duration_ms = duration.as_millis(),
            total = documents.len(),
            enriched = enriched_count,
            skipped = skipped_count,
            errors = error_count,
            "Document enrichment completed"
        );

        Ok(enriched)
    }

    /// Fetch extracted text for a document.
    #[tracing::instrument(skip(self), fields(document_id = %document_id))]
    async fn fetch_document_text(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, DocumentFetcherError> {
        let mut client = self.get_client().await?;

        // First, check if document exists and is ready
        tracing::debug!("Fetching document metadata");
        let doc_response = client
            .get_document(GetDocumentRequest {
                document_id: document_id.to_string(),
            })
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Failed to get document");
                e
            })?;

        let document = doc_response.into_inner().document.ok_or_else(|| {
            tracing::debug!("Document not found");
            DocumentFetcherError::NotFound(document_id.to_string())
        })?;

        // Check document status (3 = READY)
        if document.status != 3 {
            tracing::debug!(
                status = document.status,
                "Document not ready for text extraction"
            );
            return Ok(None);
        }

        // Check if processing metadata has extracted text
        if let Some(metadata) = document.processing_metadata {
            if let Some(text) = metadata.extracted_text {
                if !text.is_empty() {
                    tracing::debug!(
                        text_len = text.len(),
                        "Found extracted text in document metadata"
                    );
                    return Ok(Some(text));
                }
            }
        }

        // If no extracted text in document metadata, try processing status
        tracing::debug!("Checking processing status for extracted text");
        let status_response = client
            .get_processing_status(GetProcessingStatusRequest {
                document_id: document_id.to_string(),
            })
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Failed to get processing status");
                e
            })?;

        let status = status_response.into_inner();
        if let Some(metadata) = status.metadata {
            if let Some(text) = metadata.extracted_text {
                if !text.is_empty() {
                    tracing::debug!(
                        text_len = text.len(),
                        "Found extracted text in processing status"
                    );
                    return Ok(Some(text));
                }
            }
        }

        tracing::debug!("No extracted text found for document");

        Ok(None)
    }

    /// Check if a MIME type supports inline Gemini submission (images, PDFs).
    pub fn supports_inline(mime_type: &str) -> bool {
        INLINE_MIME_TYPES.contains(&mime_type)
    }

    /// Download document content via the DownloadDocument gRPC RPC.
    ///
    /// Enforces the 20MB Vertex AI inline data size limit.
    #[tracing::instrument(skip(self), fields(document_id = %document_id))]
    pub async fn download_content(
        &self,
        document_id: &str,
    ) -> Result<DownloadedContent, DocumentFetcherError> {
        let start = Instant::now();
        let mut client = self.get_client().await?;

        tracing::debug!("Starting document download via gRPC");

        let response = client
            .download_document(DownloadDocumentRequest {
                document_id: document_id.to_string(),
            })
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Failed to download document");
                e
            })?;

        let resp = response.into_inner();
        let mime_type = resp.content_type;
        let data = resp.data;

        if data.len() > MAX_INLINE_SIZE_BYTES {
            tracing::warn!(
                size = data.len(),
                max = MAX_INLINE_SIZE_BYTES,
                "Document too large for inline submission"
            );
            return Err(DocumentFetcherError::TooLarge(data.len()));
        }

        let duration = start.elapsed();

        tracing::info!(
            duration_ms = duration.as_millis(),
            size_bytes = data.len(),
            content_type = %mime_type,
            "Document download completed"
        );

        Ok(DownloadedContent { data, mime_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_fetcher_creation() {
        let fetcher = DocumentFetcher::new("http://localhost:8081");
        assert_eq!(fetcher.endpoint, "http://localhost:8081");
    }

    #[test]
    fn test_error_types() {
        assert_eq!(
            DocumentFetcherError::ConnectionError("test".to_string()).error_type(),
            "connection"
        );
        assert_eq!(
            DocumentFetcherError::NotFound("test".to_string()).error_type(),
            "not_found"
        );
        assert_eq!(
            DocumentFetcherError::NotReady("test".to_string()).error_type(),
            "not_ready"
        );
        assert_eq!(
            DocumentFetcherError::TooLarge(100).error_type(),
            "too_large"
        );
    }

    #[test]
    fn test_supports_inline_known_types() {
        assert!(DocumentFetcher::supports_inline("image/png"));
        assert!(DocumentFetcher::supports_inline("image/jpeg"));
        assert!(DocumentFetcher::supports_inline("image/gif"));
        assert!(DocumentFetcher::supports_inline("image/webp"));
        assert!(DocumentFetcher::supports_inline("image/heic"));
        assert!(DocumentFetcher::supports_inline("image/heif"));
        assert!(DocumentFetcher::supports_inline("application/pdf"));
    }

    #[test]
    fn test_supports_inline_unknown_types() {
        assert!(!DocumentFetcher::supports_inline("video/mp4"));
        assert!(!DocumentFetcher::supports_inline("audio/mpeg"));
        assert!(!DocumentFetcher::supports_inline("text/plain"));
        assert!(!DocumentFetcher::supports_inline("application/json"));
    }
}
