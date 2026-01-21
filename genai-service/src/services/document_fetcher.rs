//! Document fetcher service for retrieving document content.
//!
//! This service fetches document metadata and content from the document-service
//! via gRPC, enriching document context with extracted text when available.

use crate::grpc::document_proto::{
    document_service_client::DocumentServiceClient, GetDocumentRequest, GetProcessingStatusRequest,
};
use crate::services::providers::DocumentContext;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;

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
    async fn get_client(&self) -> Result<DocumentServiceClient<Channel>, DocumentFetcherError> {
        // Check if we already have a client
        {
            let client = self.client.read().await;
            if let Some(client) = client.as_ref() {
                return Ok(client.clone());
            }
        }

        // Create new client
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| DocumentFetcherError::ConnectionError(e.to_string()))?
            .connect()
            .await?;

        let client = DocumentServiceClient::new(channel);

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
    pub async fn enrich_documents(
        &self,
        documents: &[DocumentContext],
    ) -> Result<Vec<DocumentContext>, DocumentFetcherError> {
        let mut enriched = Vec::with_capacity(documents.len());

        for doc in documents {
            // If document already has text content, skip fetching
            if doc.text_content.is_some() {
                enriched.push(doc.clone());
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
                        url: doc.url.clone(),
                        mime_type: doc.mime_type.clone(),
                        text_content: Some(text),
                    });
                }
                Ok(None) => {
                    tracing::debug!(
                        document_id = %doc.document_id,
                        "No extracted text available for document"
                    );
                    enriched.push(doc.clone());
                }
                Err(e) => {
                    tracing::warn!(
                        document_id = %doc.document_id,
                        error = %e,
                        "Failed to fetch document text, using original context"
                    );
                    // Continue with original document context
                    enriched.push(doc.clone());
                }
            }
        }

        Ok(enriched)
    }

    /// Fetch extracted text for a document.
    async fn fetch_document_text(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, DocumentFetcherError> {
        let mut client = self.get_client().await?;

        // First, check if document exists and is ready
        let doc_response = client
            .get_document(GetDocumentRequest {
                document_id: document_id.to_string(),
            })
            .await?;

        let document = doc_response
            .into_inner()
            .document
            .ok_or_else(|| DocumentFetcherError::NotFound(document_id.to_string()))?;

        // Check document status (3 = READY)
        if document.status != 3 {
            tracing::debug!(
                document_id = %document_id,
                status = document.status,
                "Document not ready for text extraction"
            );
            return Ok(None);
        }

        // Check if processing metadata has extracted text
        if let Some(metadata) = document.processing_metadata {
            if let Some(text) = metadata.extracted_text {
                if !text.is_empty() {
                    return Ok(Some(text));
                }
            }
        }

        // If no extracted text in document metadata, try processing status
        let status_response = client
            .get_processing_status(GetProcessingStatusRequest {
                document_id: document_id.to_string(),
            })
            .await?;

        let status = status_response.into_inner();
        if let Some(metadata) = status.metadata {
            if let Some(text) = metadata.extracted_text {
                if !text.is_empty() {
                    return Ok(Some(text));
                }
            }
        }

        Ok(None)
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
}
