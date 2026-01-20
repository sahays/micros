//! Document service client for secure-frontend BFF pattern.
//!
//! Uses gRPC for internal service calls with streaming support for uploads/downloads.

use crate::config::DocumentServiceSettings;
use anyhow::Result;
use service_core::grpc::{DocumentClient as GrpcDocumentClient, DocumentClientConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Document client wrapping gRPC client for document-service communication.
pub struct DocumentClient {
    grpc_client: Arc<Mutex<GrpcDocumentClient>>,
    pub settings: DocumentServiceSettings,
}

impl DocumentClient {
    /// Create a new document client with gRPC connection.
    pub async fn new(settings: DocumentServiceSettings) -> Result<Self> {
        let config = DocumentClientConfig {
            endpoint: settings.grpc_url.clone(),
            ..Default::default()
        };

        let grpc_client = GrpcDocumentClient::new(config).await.map_err(|e| {
            tracing::error!("Failed to connect to document-service gRPC: {}", e);
            anyhow::anyhow!("gRPC connection failed: {}", e)
        })?;

        tracing::info!(
            endpoint = %settings.grpc_url,
            "Connected to document-service gRPC"
        );

        Ok(Self {
            grpc_client: Arc::new(Mutex::new(grpc_client)),
            settings,
        })
    }

    /// Upload a file to document-service via gRPC streaming.
    ///
    /// Returns the created document metadata on success.
    pub async fn upload(
        &self,
        user_id: &str,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<DocumentResponse> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .upload_document(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                filename.to_string(),
                content_type.to_string(),
                data,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Document upload failed");
                anyhow::anyhow!("Upload failed: {}", e.message())
            })?;

        let doc = response
            .document
            .ok_or_else(|| anyhow::anyhow!("No document in upload response"))?;

        Ok(DocumentResponse {
            id: doc.id,
            app_id: doc.app_id,
            org_id: doc.org_id,
            owner_id: doc.owner_id,
            original_name: doc.original_name,
            mime_type: doc.mime_type,
            size: doc.size,
            status: status_from_proto(doc.status),
            error_message: doc.error_message,
            created_at: doc.created_at.map(|t| t.seconds),
            updated_at: doc.updated_at.map(|t| t.seconds),
        })
    }

    /// Get document metadata via gRPC.
    pub async fn get_document(&self, user_id: &str, document_id: &str) -> Result<DocumentResponse> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .get_document(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                document_id.to_string(),
            )
            .await
            .map_err(|e| {
                tracing::error!(document_id = %document_id, error = %e, "Get document failed");
                anyhow::anyhow!("Get document failed: {}", e.message())
            })?;

        let doc = response
            .document
            .ok_or_else(|| anyhow::anyhow!("No document in response"))?;

        Ok(DocumentResponse {
            id: doc.id,
            app_id: doc.app_id,
            org_id: doc.org_id,
            owner_id: doc.owner_id,
            original_name: doc.original_name,
            mime_type: doc.mime_type,
            size: doc.size,
            status: status_from_proto(doc.status),
            error_message: doc.error_message,
            created_at: doc.created_at.map(|t| t.seconds),
            updated_at: doc.updated_at.map(|t| t.seconds),
        })
    }

    /// Download document content via gRPC streaming.
    ///
    /// Returns (filename, content_type, data) on success.
    pub async fn download_document(
        &self,
        user_id: &str,
        document_id: &str,
    ) -> Result<(String, String, Vec<u8>)> {
        let mut client = self.grpc_client.lock().await;

        let result = client
            .download_document(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                document_id.to_string(),
            )
            .await
            .map_err(|e| {
                tracing::error!(document_id = %document_id, error = %e, "Download failed");
                anyhow::anyhow!("Download failed: {}", e.message())
            })?;

        Ok(result)
    }

    /// List documents for a user via gRPC.
    pub async fn list_documents(
        &self,
        user_id: &str,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<DocumentListResponse> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .list_documents(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                None, // status filter
                None, // mime_type filter
                page,
                page_size,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "List documents failed");
                anyhow::anyhow!("List documents failed: {}", e.message())
            })?;

        let documents = response
            .documents
            .into_iter()
            .map(|doc| DocumentResponse {
                id: doc.id,
                app_id: doc.app_id,
                org_id: doc.org_id,
                owner_id: doc.owner_id,
                original_name: doc.original_name,
                mime_type: doc.mime_type,
                size: doc.size,
                status: status_from_proto(doc.status),
                error_message: doc.error_message,
                created_at: doc.created_at.map(|t| t.seconds),
                updated_at: doc.updated_at.map(|t| t.seconds),
            })
            .collect();

        Ok(DocumentListResponse {
            documents,
            total: response.total,
            page: response.page,
            page_size: response.page_size,
            total_pages: response.total_pages,
        })
    }

    /// Delete a document via gRPC.
    pub async fn delete_document(&self, user_id: &str, document_id: &str) -> Result<bool> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .delete_document(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                document_id.to_string(),
            )
            .await
            .map_err(|e| {
                tracing::error!(document_id = %document_id, error = %e, "Delete failed");
                anyhow::anyhow!("Delete failed: {}", e.message())
            })?;

        Ok(response.success)
    }

    /// Generate a signed URL for document download via gRPC.
    pub async fn generate_signed_url(
        &self,
        user_id: &str,
        document_id: &str,
        expires_in_seconds: i64,
    ) -> Result<SignedUrlResponse> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .generate_signed_url(
                &self.settings.default_app_id,
                &self.settings.default_org_id,
                user_id,
                document_id.to_string(),
                expires_in_seconds,
            )
            .await
            .map_err(|e| {
                tracing::error!(document_id = %document_id, error = %e, "Generate signed URL failed");
                anyhow::anyhow!("Generate signed URL failed: {}", e.message())
            })?;

        Ok(SignedUrlResponse {
            url: response.url,
            expires_at: response.expires_at.map(|t| t.seconds),
        })
    }
}

/// Convert proto document status to string.
fn status_from_proto(status: i32) -> String {
    match status {
        1 => "uploading".to_string(),
        2 => "processing".to_string(),
        3 => "ready".to_string(),
        4 => "failed".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Document metadata response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub app_id: String,
    pub org_id: String,
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Document list response with pagination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}

/// Signed URL response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedUrlResponse {
    pub url: String,
    pub expires_at: Option<i64>,
}
