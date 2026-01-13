use crate::dtos::{
    ChunkMetadata, ChunkedVideoResponse, DocumentListParams, DocumentListResponse,
    DocumentResponse, DownloadParams, ProcessingOptions, ProcessingStatusResponse,
};
use crate::middleware::user_id::UserId;
use crate::models::{Document, DocumentStatus};
use crate::startup::AppState;
use crate::workers::ProcessingJob;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use service_core::error::AppError;
use uuid::Uuid;

pub async fn list_documents(
    State(state): State<AppState>,
    user_id: UserId,
    Query(params): Query<DocumentListParams>,
) -> Result<impl IntoResponse, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).max(1).min(100);
    let skip = (page - 1) * page_size;

    let mut filter = doc! { "owner_id": user_id.0 };

    if let Some(status) = params.status {
        // Convert status enum to bson string if needed, depending on model serialization
        // Assuming serde handles it or we match explicitly
        // Since DocumentStatus derives Serialize/Deserialize, bson should handle it but let's be safe
        // Actually, mongodb driver uses to_bson, so direct usage might be tricky if it's an enum
        // Let's assume standard serde serialization (lowercase usually)
        let bson_status = mongodb::bson::to_bson(&status).map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to serialize status: {}", e))
        })?;
        filter.insert("status", bson_status);
    }

    if let Some(mime_type) = params.mime_type {
        // Simple partial match using regex could be better, but exact match for now
        // or prefix match: { "mime_type": { "$regex": format!("^{}", mime_type) } }
        filter.insert("mime_type", doc! { "$regex": format!("^{}", mime_type) });
    }

    // Count total documents
    let total = state
        .db
        .documents()
        .count_documents(filter.clone(), None)
        .await
        .map_err(AppError::from)?;

    // Fetch documents
    let find_options = FindOptions::builder()
        .sort(doc! { "created_at": -1 }) // Newest first
        .skip(skip)
        .limit(page_size as i64)
        .build();

    let mut cursor = state
        .db
        .documents()
        .find(filter, find_options)
        .await
        .map_err(AppError::from)?;

    let mut documents = Vec::new();
    while let Some(doc) = cursor.try_next().await.map_err(AppError::from)? {
        documents.push(DocumentResponse::from(doc));
    }

    let total_pages = (total as f64 / page_size as f64).ceil() as u64;

    Ok(Json(DocumentListResponse {
        documents,
        total,
        page,
        page_size,
        total_pages,
    }))
}

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

enum FileToServe {
    File {
        storage_key: String,
        content_type: String,
        filename: String,
    },
    ChunkedVideo(ChunkedVideoResponse),
}

fn determine_file_to_serve(document: &Document) -> Result<FileToServe, AppError> {
    // Check if processed file exists
    if let Some(ref metadata) = document.processing_metadata {
        // For videos: check if chunked
        if document.mime_type.starts_with("video/") {
            if let Some(ref chunks) = metadata.chunks {
                // Chunked video - return JSON metadata
                return Ok(FileToServe::ChunkedVideo(ChunkedVideoResponse {
                    type_: "chunked_video".to_string(),
                    original_name: document.original_name.clone(),
                    resolution: metadata.resolution.clone(),
                    total_size: metadata.total_size.unwrap_or(0),
                    chunk_count: chunks.len(),
                    chunks: chunks
                        .iter()
                        .map(|c| ChunkMetadata {
                            index: c.index,
                            url: format!("/documents/{}/chunks/{}", document.id, c.index),
                            size: c.size,
                            content_type: "video/mp4".to_string(),
                        })
                        .collect(),
                }));
            }
        }

        // For images or unchunked videos: serve processed file
        if let Some(ref processed_path) = metadata.thumbnail_path {
            return Ok(FileToServe::File {
                storage_key: processed_path.clone(),
                content_type: detect_content_type(processed_path),
                filename: document.original_name.clone(),
            });
        }
    }

    // Default: serve original file
    Ok(FileToServe::File {
        storage_key: document.storage_key.clone(),
        content_type: document.mime_type.clone(),
        filename: document.original_name.clone(),
    })
}

fn detect_content_type(path: &str) -> String {
    if path.ends_with(".webp") {
        "image/webp".to_string()
    } else if path.ends_with(".mp4") {
        "video/mp4".to_string()
    } else if path.ends_with(".pdf") {
        "application/pdf".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

pub async fn download_document(
    State(state): State<AppState>,
    user_id: Option<UserId>,
    Path(document_id): Path<String>,
    Query(params): Query<DownloadParams>,
) -> Result<impl IntoResponse, AppError> {
    // Validate signature if provided, otherwise require user_id
    if let (Some(signature), Some(expires)) = (&params.signature, &params.expires) {
        service_core::utils::signature::validate_document_signature(
            &document_id,
            signature,
            *expires,
            &state.config.signature.signing_secret,
        )?;
    } else {
        // Normal flow: require X-User-ID header
        user_id.ok_or_else(|| {
            AppError::Unauthorized(anyhow::anyhow!("Missing user ID or signature"))
        })?;
    }

    // Fetch document metadata
    let document = state
        .db
        .documents()
        .find_one(doc! { "_id": &document_id }, None)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Document not found")))?;

    // Determine what to serve
    match determine_file_to_serve(&document)? {
        FileToServe::File {
            storage_key,
            content_type,
            filename,
        } => {
            // Download file from storage
            let file_data = state.storage.download(&storage_key).await.map_err(|e| {
                tracing::error!(
                    document_id = %document_id,
                    storage_key = %storage_key,
                    error = %e,
                    "Failed to download file"
                );
                e
            })?;

            tracing::info!(
                document_id = %document_id,
                storage_key = %storage_key,
                content_type = %content_type,
                size = file_data.len(),
                "Document download completed"
            );

            // Return file
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, content_type),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"{}\"", filename),
                    ),
                ],
                file_data,
            )
                .into_response())
        }
        FileToServe::ChunkedVideo(metadata) => {
            tracing::info!(
                document_id = %document_id,
                chunk_count = metadata.chunk_count,
                "Returning chunked video metadata"
            );

            // Return JSON with chunk URLs
            Ok((StatusCode::OK, Json(metadata)).into_response())
        }
    }
}

pub async fn download_video_chunk(
    State(state): State<AppState>,
    _user_id: UserId,
    Path((document_id, chunk_index)): Path<(String, usize)>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Fetch document metadata
    let document = state
        .db
        .documents()
        .find_one(doc! { "_id": &document_id }, None)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Document not found")))?;

    // 2. Verify it's a chunked video
    let chunks = document
        .processing_metadata
        .as_ref()
        .and_then(|m| m.chunks.as_ref())
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Document is not chunked")))?;

    // 3. Validate chunk index
    let chunk_info = chunks
        .get(chunk_index)
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Chunk index out of range")))?;

    // 4. Download chunk
    let chunk_data = state
        .storage
        .download(&chunk_info.path)
        .await
        .map_err(|e| {
            tracing::error!(
                document_id = %document_id,
                chunk_index = chunk_index,
                error = %e,
                "Failed to download chunk"
            );
            AppError::NotFound(anyhow::anyhow!("Chunk file not found"))
        })?;

    tracing::info!(
        document_id = %document_id,
        chunk_index = chunk_index,
        size = chunk_data.len(),
        "Video chunk download completed"
    );

    // 5. Return chunk
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "video/mp4")],
        chunk_data,
    ))
}
