use crate::dtos::{DocumentResponse, ProcessingOptions, ProcessingStatusResponse};
use crate::middleware::user_id::UserId;
use crate::models::{Document, DocumentStatus};
use crate::startup::AppState;
use crate::workers::ProcessingJob;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use mongodb::bson::doc;
use service_core::error::AppError;
use uuid::Uuid;

pub async fn upload_document(
    State(state): State<AppState>,
    user_id: UserId,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| {
            AppError::BadRequest(anyhow::anyhow!("Failed to read multipart field: {}", e))
        })?
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("No file uploaded")))?;

    let original_name = field.file_name().unwrap_or("unnamed").to_string();
    let mime_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    // Read the entire file into memory for now (simpler for prototype)
    // For large files, implement streaming/chunked uploads
    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(anyhow::anyhow!("Failed to read file bytes: {}", e)))?
        .to_vec();

    let size = data.len() as i64;

    // Check size limit (e.g., 5MB as per AC, but let's make it 20MB for now)
    if size > 20 * 1024 * 1024 {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "File too large (max 20MB)"
        )));
    }

    // Generate unique storage key
    let extension = std::path::Path::new(&original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");

    let storage_key = format!("{}/{}.{}", Uuid::new_v4(), Uuid::new_v4(), extension);

    // Create document metadata with actual user_id from BFF (secure-frontend)
    // user_id propagated via X-User-ID header in HMAC-signed request
    let mut document = Document::new(
        user_id.0,
        original_name,
        mime_type,
        size,
        storage_key.clone(),
    );

    tracing::info!(
        document_id = %document.id,
        filename = %document.original_name,
        size = %size,
        "Document upload started"
    );

    // 1. Upload to storage
    state
        .storage
        .upload(&storage_key, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload file {} to storage: {}", storage_key, e);
            e
        })?;

    // 2. Set status to Ready and save to DB (processing must be triggered manually)
    document.status = DocumentStatus::Ready;

    state
        .db
        .documents()
        .insert_one(&document, None)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to insert document {} into database: {}",
                document.id,
                e
            );
            AppError::from(e)
        })?;

    tracing::info!(
        document_id = %document.id,
        "Document upload completed successfully"
    );

    Ok((StatusCode::CREATED, Json(DocumentResponse::from(document))))
}

pub async fn process_document(
    State(state): State<AppState>,
    _user_id: UserId, // Available for logging/auditing, but authorization is BFF's responsibility
    Path(document_id): Path<String>,
    Json(options): Json<ProcessingOptions>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch document from database
    let document = state
        .db
        .documents()
        .find_one(doc! { "_id": &document_id }, None)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Document not found")))?;

    // 2. Check if document is in a processable state
    // Note: Ownership validation is the BFF's responsibility (secure-frontend)
    if matches!(document.status, DocumentStatus::Processing) {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Document is already being processed"
        )));
    }

    // 3. Update status to Processing
    state
        .db
        .documents()
        .update_one(
            doc! { "_id": &document_id },
            doc! { "$set": { "status": "processing" } },
            None,
        )
        .await
        .map_err(AppError::from)?;

    // 4. Enqueue processing job
    if let Some(job_tx) = &state.job_tx {
        let job = ProcessingJob {
            document_id: document.id.clone(),
            owner_id: document.owner_id.clone(),
            mime_type: document.mime_type.clone(),
            storage_key: document.storage_key.clone(),
            options,
        };

        job_tx.send(job).await.map_err(|_| {
            tracing::error!(document_id = %document.id, "Failed to enqueue processing job");
            AppError::InternalError(anyhow::anyhow!("Worker queue is full"))
        })?;

        tracing::info!(document_id = %document.id, "Processing job enqueued");
    } else {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "Worker pool not available"
        )));
    }

    Ok(StatusCode::ACCEPTED)
}

pub async fn get_document_status(
    State(state): State<AppState>,
    _user_id: UserId, // Available for logging/auditing, but authorization is BFF's responsibility
    Path(document_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch document from database
    // Note: Ownership validation is the BFF's responsibility (secure-frontend)
    let document = state
        .db
        .documents()
        .find_one(doc! { "_id": &document_id }, None)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Document not found")))?;

    // 2. Convert to status response
    let status_response = ProcessingStatusResponse::from(document);

    Ok(Json(status_response))
}
