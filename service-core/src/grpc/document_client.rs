//! Document service gRPC client for service-to-service communication.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::document::document_service_client::DocumentServiceClient;
use super::proto::document::{
    DeleteDocumentRequest, DeleteDocumentResponse, DownloadDocumentRequest,
    GenerateSignedUrlRequest, GenerateSignedUrlResponse, GetDocumentRequest, GetDocumentResponse,
    GetProcessingStatusRequest, GetProcessingStatusResponse, ListDocumentsRequest,
    ListDocumentsResponse, ProcessDocumentRequest, ProcessDocumentResponse, ProcessingOptions,
    UploadDocumentRequest, UploadDocumentResponse,
};

/// Configuration for the document service client.
#[derive(Clone, Debug)]
pub struct DocumentClientConfig {
    /// The gRPC endpoint of the document service (e.g., "http://document-service:8081").
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
}

impl Default for DocumentClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50053".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(60), // Longer timeout for file operations
        }
    }
}

/// Document service client for calling document-service via gRPC.
#[derive(Clone)]
pub struct DocumentClient {
    client: DocumentServiceClient<Channel>,
}

impl DocumentClient {
    /// Create a new document client with the given configuration.
    pub async fn new(config: DocumentClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            client: DocumentServiceClient::new(channel),
        })
    }

    /// Create a new document client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(DocumentClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    // =========================================================================
    // Upload
    // =========================================================================

    /// Upload a document using streaming.
    ///
    /// This method takes the file data and metadata, then streams the upload
    /// to the document service.
    pub async fn upload_document(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        filename: String,
        mime_type: String,
        data: Vec<u8>,
    ) -> Result<UploadDocumentResponse, tonic::Status> {
        use super::proto::document::UploadMetadata;
        use super::proto::document::upload_document_request::Data;
        use std::collections::HashMap;

        // Create metadata message
        let metadata_msg = UploadDocumentRequest {
            data: Some(Data::Metadata(UploadMetadata {
                filename,
                mime_type,
                metadata: HashMap::new(),
            })),
        };

        // Create chunk messages
        let chunk_size = 64 * 1024; // 64KB chunks
        let mut messages = vec![metadata_msg];

        for chunk in data.chunks(chunk_size) {
            messages.push(UploadDocumentRequest {
                data: Some(Data::Chunk(chunk.to_vec())),
            });
        }

        // Create request with tenant metadata
        let mut request = Request::new(futures::stream::iter(messages));
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.upload_document(request).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Download
    // =========================================================================

    /// Download a document and return the complete data.
    ///
    /// This method handles the streaming response and collects all chunks.
    pub async fn download_document(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<(String, String, Vec<u8>), tonic::Status> {
        use super::proto::document::download_document_response::Data;
        use futures::StreamExt;

        let mut request = Request::new(DownloadDocumentRequest {
            document_id,
            signature: None,
            expires: None,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.download_document(request).await?;
        let mut stream = response.into_inner();

        let mut filename = String::new();
        let mut content_type = String::new();
        let mut file_data = Vec::new();

        while let Some(msg) = stream.next().await {
            let msg = msg?;
            match msg.data {
                Some(Data::Metadata(m)) => {
                    filename = m.filename;
                    content_type = m.content_type;
                }
                Some(Data::Chunk(chunk)) => {
                    file_data.extend_from_slice(&chunk);
                }
                Some(Data::ChunkedVideo(_)) => {
                    return Err(tonic::Status::failed_precondition(
                        "Document is a chunked video, use download_video_chunk instead",
                    ));
                }
                None => {}
            }
        }

        Ok((filename, content_type, file_data))
    }

    /// Download a document with a signed URL (no tenant context needed).
    pub async fn download_document_signed(
        &mut self,
        document_id: String,
        signature: String,
        expires: i64,
    ) -> Result<(String, String, Vec<u8>), tonic::Status> {
        use super::proto::document::download_document_response::Data;
        use futures::StreamExt;

        let request = Request::new(DownloadDocumentRequest {
            document_id,
            signature: Some(signature),
            expires: Some(expires),
        });

        let response = self.client.download_document(request).await?;
        let mut stream = response.into_inner();

        let mut filename = String::new();
        let mut content_type = String::new();
        let mut file_data = Vec::new();

        while let Some(msg) = stream.next().await {
            let msg = msg?;
            match msg.data {
                Some(Data::Metadata(m)) => {
                    filename = m.filename;
                    content_type = m.content_type;
                }
                Some(Data::Chunk(chunk)) => {
                    file_data.extend_from_slice(&chunk);
                }
                Some(Data::ChunkedVideo(_)) => {
                    return Err(tonic::Status::failed_precondition(
                        "Document is a chunked video, use download_video_chunk instead",
                    ));
                }
                None => {}
            }
        }

        Ok((filename, content_type, file_data))
    }

    // =========================================================================
    // Metadata
    // =========================================================================

    /// Get document metadata by ID.
    pub async fn get_document(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<GetDocumentResponse, tonic::Status> {
        let mut request = Request::new(GetDocumentRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.get_document(request).await?;
        Ok(response.into_inner())
    }

    /// List documents with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_documents(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        status: Option<i32>,
        mime_type: Option<String>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<ListDocumentsResponse, tonic::Status> {
        let mut request = Request::new(ListDocumentsRequest {
            status,
            mime_type,
            page,
            page_size,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.list_documents(request).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Delete
    // =========================================================================

    /// Delete a document by ID.
    pub async fn delete_document(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<DeleteDocumentResponse, tonic::Status> {
        let mut request = Request::new(DeleteDocumentRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.delete_document(request).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Processing
    // =========================================================================

    /// Request processing of a document.
    pub async fn process_document(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
        options: Option<ProcessingOptions>,
    ) -> Result<ProcessDocumentResponse, tonic::Status> {
        let mut request = Request::new(ProcessDocumentRequest {
            document_id,
            options,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.process_document(request).await?;
        Ok(response.into_inner())
    }

    /// Get the processing status of a document.
    pub async fn get_processing_status(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<GetProcessingStatusResponse, tonic::Status> {
        let mut request = Request::new(GetProcessingStatusRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.get_processing_status(request).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Signed URLs
    // =========================================================================

    /// Generate a signed URL for document access.
    pub async fn generate_signed_url(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
        document_id: String,
        expires_in_seconds: i64,
    ) -> Result<GenerateSignedUrlResponse, tonic::Status> {
        let mut request = Request::new(GenerateSignedUrlRequest {
            document_id,
            expires_in_seconds,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.generate_signed_url(request).await?;
        Ok(response.into_inner())
    }
}

// Re-export document proto types for convenience
pub use super::proto::document::{
    Document as DocumentProto, DocumentStatus as DocumentStatusProto,
    ProcessingMetadata as ProcessingMetadataProto, ProcessingOptions as ProcessingOptionsProto,
    ProcessorType as ProcessorTypeProto,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_client_config_default() {
        let config = DocumentClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50053");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(60));
    }
}
