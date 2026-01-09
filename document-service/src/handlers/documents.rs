use axum::{
    extract::{Multipart, State},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use crate::startup::AppState;
use crate::models::{Document, DocumentStatus};
use crate::dtos::DocumentResponse;
use service_core::error::AppError;
use uuid::Uuid;

pub async fn upload_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(anyhow::anyhow!("Failed to read multipart field: {}", e)))?
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("No file uploaded")))?;

    let original_name = field.file_name().unwrap_or("unnamed").to_string();
    let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();
    
    // Read the entire file into memory for now (simpler for prototype)
    // For large files, we should stream it to S3
    let data = field.bytes().await
        .map_err(|e| AppError::BadRequest(anyhow::anyhow!("Failed to read file bytes: {}", e)))?
        .to_vec();
    
    let size = data.len() as i64;
    
    // Check size limit (e.g., 5MB as per AC, but let's make it 20MB for now)
    if size > 20 * 1024 * 1024 {
        return Err(AppError::BadRequest(anyhow::anyhow!("File too large (max 20MB)")));
    }

    // Generate unique storage key
    let extension = std::path::Path::new(&original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    
    let s3_key = format!("{}/{}.{}", Uuid::new_v4(), Uuid::new_v4(), extension);

    // Create document metadata
    // owner_id is hardcoded for now until we have auth middleware
    let owner_id = "test_owner".to_string();
    let mut document = Document::new(
        owner_id,
        original_name,
        mime_type,
        size,
        s3_key.clone(),
    );

    // 1. Upload to storage
    state.storage.upload(&s3_key, data).await?;

    // 2. Update status and save to DB
    document.status = DocumentStatus::Ready;
    
    state.db.documents().insert_one(&document, None).await?;

    Ok((StatusCode::CREATED, Json(DocumentResponse::from(document))))
}
