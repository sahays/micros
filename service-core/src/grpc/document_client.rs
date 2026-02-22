//! Document service gRPC client for service-to-service communication.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::document::document_service_client::DocumentServiceClient;
use super::proto::document::{
    DeleteDocumentRequest, DeleteDocumentResponse, DownloadDocumentRequest,
    GetDocumentRequest, GetDocumentResponse,
    GetProcessingStatusRequest, GetProcessingStatusResponse, ListDocumentsRequest,
    ListDocumentsResponse, PdfOptions, ProcessDocumentRequest, ProcessDocumentResponse,
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

    /// Upload a document.
    pub async fn upload_document(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: &str,
        filename: String,
        mime_type: String,
        data: Vec<u8>,
    ) -> Result<UploadDocumentResponse, tonic::Status> {
        let mut request = Request::new(UploadDocumentRequest {
            filename,
            mime_type,
            data,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
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
    pub async fn download_document(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<(String, String, Vec<u8>), tonic::Status> {
        let mut request = Request::new(DownloadDocumentRequest {
            document_id,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.download_document(request).await?;
        let resp = response.into_inner();

        Ok((resp.filename, resp.content_type, resp.data))
    }

    // =========================================================================
    // Metadata
    // =========================================================================

    /// Get document metadata by ID.
    pub async fn get_document(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<GetDocumentResponse, tonic::Status> {
        let mut request = Request::new(GetDocumentRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
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
        tenant_id: &str,
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
            .insert("x-tenant-id", tenant_id.parse().unwrap());
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
        tenant_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<DeleteDocumentResponse, tonic::Status> {
        let mut request = Request::new(DeleteDocumentRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
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
        tenant_id: &str,
        user_id: &str,
        document_id: String,
        pdf_options: Option<PdfOptions>,
    ) -> Result<ProcessDocumentResponse, tonic::Status> {
        let mut request = Request::new(ProcessDocumentRequest {
            document_id,
            pdf_options,
        });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
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
        tenant_id: &str,
        user_id: &str,
        document_id: String,
    ) -> Result<GetProcessingStatusResponse, tonic::Status> {
        let mut request = Request::new(GetProcessingStatusRequest { document_id });
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());

        let response = self.client.get_processing_status(request).await?;
        Ok(response.into_inner())
    }
}

// Re-export document proto types for convenience
pub use super::proto::document::{
    Document as DocumentProto, DocumentStatus as DocumentStatusProto,
    ProcessingMetadata as ProcessingMetadataProto,
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
