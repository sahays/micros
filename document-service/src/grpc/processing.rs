use crate::grpc::document_service::{extract_tenant_context, status_to_proto};
use crate::grpc::proto::{
    DocumentStatus, GetProcessingStatusRequest, GetProcessingStatusResponse,
    ProcessDocumentRequest, ProcessDocumentResponse, ProcessingMetadata as ProtoProcessingMetadata,
};
use crate::models::DocumentStatus as ModelDocumentStatus;
use crate::startup::AppState;
use crate::workers::ProcessingJob;
use mongodb::bson::doc;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn process_document(
    state: &AppState,
    request: Request<ProcessDocumentRequest>,
) -> Result<Response<ProcessDocumentResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Fetch document
    let document = state
        .db
        .documents()
        .find_one(
            doc! {
                "_id": &req.document_id,
                "app_id": &tenant.app_id,
                "org_id": &tenant.org_id
            },
            None,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found("Document not found"))?;

    // Check if already processing
    if matches!(document.status, ModelDocumentStatus::Processing) {
        return Err(Status::failed_precondition(
            "Document is already being processed",
        ));
    }

    // Update status
    state
        .db
        .documents()
        .update_one(
            doc! { "_id": &req.document_id },
            doc! { "$set": { "status": "processing" } },
            None,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    // Enqueue job
    if let Some(job_tx) = &state.job_tx {
        let job = ProcessingJob {
            document_id: document.id.clone(),
            app_id: document.app_id.clone(),
            org_id: document.org_id.clone(),
            owner_id: document.owner_id.clone(),
            mime_type: document.mime_type.clone(),
            storage_key: document.storage_key.clone(),
        };

        job_tx.send(job).await.map_err(|_| {
            tracing::error!(document_id = %document.id, "Failed to enqueue processing job");
            Status::internal("Worker queue is full")
        })?;

        tracing::info!(document_id = %document.id, "Processing job enqueued via gRPC");
    } else {
        return Err(Status::unavailable("Worker pool not available"));
    }

    Ok(Response::new(ProcessDocumentResponse {
        queued: true,
        status: DocumentStatus::Processing as i32,
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn get_processing_status(
    state: &AppState,
    request: Request<GetProcessingStatusRequest>,
) -> Result<Response<GetProcessingStatusResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let document = state
        .db
        .documents()
        .find_one(
            doc! {
                "_id": &req.document_id,
                "app_id": &tenant.app_id,
                "org_id": &tenant.org_id
            },
            None,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found("Document not found"))?;

    Ok(Response::new(GetProcessingStatusResponse {
        document_id: document.id,
        status: status_to_proto(&document.status),
        metadata: document
            .processing_metadata
            .map(|m| ProtoProcessingMetadata {
                extracted_text: m.extracted_text,
                page_count: m.page_count,
                error_details: m.error_details,
            }),
        error_message: document.error_message,
    }))
}
