use crate::grpc::capability_check::capabilities;
use crate::grpc::document_service::{
    datetime_to_timestamp, extract_tenant_context, proto_to_processing_options, status_to_proto,
};
use crate::grpc::proto::{
    DocumentStatus, GenerateSignedUrlRequest, GenerateSignedUrlResponse,
    GetProcessingStatusRequest, GetProcessingStatusResponse, ProcessDocumentRequest,
    ProcessDocumentResponse, ProcessingMetadata as ProtoProcessingMetadata, ProcessingProgress,
};
use crate::models::DocumentStatus as ModelDocumentStatus;
use crate::startup::AppState;
use crate::workers::ProcessingJob;
use metrics::counter;
use mongodb::bson::doc;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn process_document(
    state: &AppState,
    request: Request<ProcessDocumentRequest>,
) -> Result<Response<ProcessDocumentResponse>, Status> {
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_PROCESS)
        .await?;

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
        let options = proto_to_processing_options(req.options);
        let job = ProcessingJob {
            document_id: document.id.clone(),
            app_id: document.app_id.clone(),
            org_id: document.org_id.clone(),
            owner_id: document.owner_id.clone(),
            mime_type: document.mime_type.clone(),
            storage_key: document.storage_key.clone(),
            options,
        };

        job_tx.send(job).await.map_err(|_| {
            tracing::error!(document_id = %document.id, "Failed to enqueue processing job");
            Status::internal("Worker queue is full")
        })?;

        // Record metering metrics
        let labels = [
            ("tenant_id", tenant.app_id.clone()),
            ("mime_type", document.mime_type.clone()),
        ];
        counter!("document_processing_requests_total", &labels).increment(1);

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
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_READ)
        .await?;

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

    let progress = if matches!(document.status, ModelDocumentStatus::Processing) {
        Some(ProcessingProgress {
            current_processor: None,
            processors_completed: vec![],
            processors_remaining: vec![],
            percent_complete: 0,
        })
    } else {
        None
    };

    Ok(Response::new(GetProcessingStatusResponse {
        document_id: document.id,
        status: status_to_proto(&document.status),
        progress,
        metadata: document
            .processing_metadata
            .map(|m| ProtoProcessingMetadata {
                extracted_text: m.extracted_text,
                page_count: m.page_count,
                duration_seconds: m.duration_seconds,
                optimized_size: m.optimized_size,
                thumbnail_path: m.thumbnail_path,
                error_details: m.error_details,
                resolution: m.resolution,
                chunk_count: m.chunk_count,
                total_size: m.total_size,
            }),
        error_message: document.error_message,
        processing_attempts: document.processing_attempts,
        last_processing_attempt: document.last_processing_attempt.map(|dt| Timestamp {
            seconds: dt.timestamp_millis() / 1000,
            nanos: ((dt.timestamp_millis() % 1000) * 1_000_000) as i32,
        }),
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn generate_signed_url(
    state: &AppState,
    request: Request<GenerateSignedUrlRequest>,
) -> Result<Response<GenerateSignedUrlResponse>, Status> {
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_SIGNED_URL)
        .await?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Verify document exists and belongs to tenant
    let _document = state
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

    let expires_in = req.expires_in_seconds.clamp(60, 86400); // 1 min to 24 hours
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
    let expires_timestamp = expires_at.timestamp();

    let signature = service_core::utils::signature::generate_document_signature(
        &req.document_id,
        expires_timestamp,
        &state.config.signature.signing_secret,
    )
    .map_err(|e| Status::internal(format!("Failed to generate signature: {}", e)))?;

    // Note: The actual URL construction depends on your deployment
    // This is a placeholder that returns the signature and expiry
    let url = format!(
        "/documents/{}?signature={}&expires={}",
        req.document_id, signature, expires_timestamp
    );

    Ok(Response::new(GenerateSignedUrlResponse {
        url,
        expires_at: Some(datetime_to_timestamp(expires_at)),
    }))
}
