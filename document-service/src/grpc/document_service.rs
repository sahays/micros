use crate::grpc::proto::{
    document_service_server::DocumentService, DeleteDocumentRequest, DeleteDocumentResponse,
    Document as ProtoDocument, DocumentStatus, DownloadDocumentRequest, DownloadDocumentResponse,
    GetDocumentRequest, GetDocumentResponse, GetProcessingStatusRequest,
    GetProcessingStatusResponse, ListDocumentsRequest, ListDocumentsResponse,
    ProcessDocumentRequest, ProcessDocumentResponse, ProcessingMetadata as ProtoProcessingMetadata,
    UploadDocumentRequest, UploadDocumentResponse,
};
use crate::middleware::tenant::TenantContext;
use crate::models::{Document, DocumentStatus as ModelDocumentStatus};
use crate::startup::AppState;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

use super::{documents, processing};

pub struct DocumentGrpcService {
    state: AppState,
}

impl DocumentGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Extract tenant context from gRPC metadata.
#[allow(clippy::result_large_err)]
pub fn extract_tenant_context(
    request: &Request<impl std::any::Any>,
) -> Result<TenantContext, Status> {
    let metadata = request.metadata();

    let app_id = metadata
        .get("x-app-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;

    let org_id = metadata
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Status::unauthenticated("Missing x-org-id header"))?;

    let user_id = metadata
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Status::unauthenticated("Missing x-user-id header"))?;

    Ok(TenantContext {
        app_id,
        org_id,
        user_id,
    })
}

// Conversion helpers
pub fn status_to_proto(status: &ModelDocumentStatus) -> i32 {
    match status {
        ModelDocumentStatus::Uploading => DocumentStatus::Uploading as i32,
        ModelDocumentStatus::Processing => DocumentStatus::Processing as i32,
        ModelDocumentStatus::Ready => DocumentStatus::Ready as i32,
        ModelDocumentStatus::Failed => DocumentStatus::Failed as i32,
    }
}

pub fn proto_to_status(status: i32) -> Option<ModelDocumentStatus> {
    match DocumentStatus::try_from(status) {
        Ok(DocumentStatus::Uploading) => Some(ModelDocumentStatus::Uploading),
        Ok(DocumentStatus::Processing) => Some(ModelDocumentStatus::Processing),
        Ok(DocumentStatus::Ready) => Some(ModelDocumentStatus::Ready),
        Ok(DocumentStatus::Failed) => Some(ModelDocumentStatus::Failed),
        _ => None,
    }
}

pub fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

pub fn document_to_proto(doc: &Document) -> ProtoDocument {
    ProtoDocument {
        id: doc.id.clone(),
        app_id: doc.app_id.clone(),
        org_id: doc.org_id.clone(),
        owner_id: doc.owner_id.clone(),
        original_name: doc.original_name.clone(),
        mime_type: doc.mime_type.clone(),
        size: doc.size,
        status: status_to_proto(&doc.status),
        error_message: doc.error_message.clone(),
        processing_metadata: doc
            .processing_metadata
            .as_ref()
            .map(|m| ProtoProcessingMetadata {
                extracted_text: m.extracted_text.clone(),
                page_count: m.page_count,
                error_details: m.error_details.clone(),
            }),
        created_at: Some(datetime_to_timestamp(doc.created_at)),
        updated_at: Some(datetime_to_timestamp(doc.updated_at)),
    }
}

#[tonic::async_trait]
impl DocumentService for DocumentGrpcService {
    #[tracing::instrument(skip(self, request))]
    async fn upload_document(
        &self,
        request: Request<UploadDocumentRequest>,
    ) -> Result<Response<UploadDocumentResponse>, Status> {
        documents::upload_document(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn download_document(
        &self,
        request: Request<DownloadDocumentRequest>,
    ) -> Result<Response<DownloadDocumentResponse>, Status> {
        documents::download_document(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_document(
        &self,
        request: Request<GetDocumentRequest>,
    ) -> Result<Response<GetDocumentResponse>, Status> {
        documents::get_document(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn list_documents(
        &self,
        request: Request<ListDocumentsRequest>,
    ) -> Result<Response<ListDocumentsResponse>, Status> {
        documents::list_documents(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn delete_document(
        &self,
        request: Request<DeleteDocumentRequest>,
    ) -> Result<Response<DeleteDocumentResponse>, Status> {
        documents::delete_document(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn process_document(
        &self,
        request: Request<ProcessDocumentRequest>,
    ) -> Result<Response<ProcessDocumentResponse>, Status> {
        processing::process_document(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_processing_status(
        &self,
        request: Request<GetProcessingStatusRequest>,
    ) -> Result<Response<GetProcessingStatusResponse>, Status> {
        processing::get_processing_status(&self.state, request).await
    }
}
