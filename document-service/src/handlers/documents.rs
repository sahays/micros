use crate::dtos::DocumentResponse;
use crate::middleware::user_id::UserId;
use crate::models::{Document, DocumentStatus};
use crate::startup::AppState;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
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

    // 2. Update status and save to DB
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

    Ok((StatusCode::CREATED, Json(DocumentResponse::from(document))))
}
