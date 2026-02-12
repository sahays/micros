use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::document_service::DownloadStream;
use crate::grpc::document_service::{
    document_to_proto, extract_tenant_context, extract_tenant_context_from_streaming,
    proto_to_status, CHUNK_SIZE,
};
use crate::grpc::proto::{
    ChunkedVideoInfo, DeleteDocumentRequest, DeleteDocumentResponse, DownloadDocumentRequest,
    DownloadDocumentResponse, DownloadMetadata, GetDocumentRequest, GetDocumentResponse,
    ListDocumentsRequest, ListDocumentsResponse, UploadDocumentRequest, UploadDocumentResponse,
    VideoChunkInfo,
};
use crate::models::{Document, DocumentStatus as ModelDocumentStatus};
use crate::startup::AppState;
use futures::stream::TryStreamExt;
use futures::StreamExt;
use metrics::{counter, histogram};
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

#[tracing::instrument(skip(state, request))]
pub async fn upload_document(
    state: &AppState,
    request: Request<Streaming<UploadDocumentRequest>>,
) -> Result<Response<UploadDocumentResponse>, Status> {
    // Extract metadata for capability check before consuming the request
    let cap_metadata = CapabilityMetadata::try_from_request(&request);
    let tenant = extract_tenant_context_from_streaming(&request)?;
    let mut stream = request.into_inner();

    // Capability check (if enabled) - use extracted metadata
    if let Some(metadata) = cap_metadata {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::DOCUMENT_UPLOAD)
            .await?;
    } else if state.capability_checker.is_enabled() {
        // If capability checking is enabled but no auth header, fail
        return Err(Status::unauthenticated("Missing authorization header"));
    }

    // First message should contain metadata
    let first_msg = stream
        .next()
        .await
        .ok_or_else(|| Status::invalid_argument("Empty upload stream"))?
        .map_err(|e| Status::internal(format!("Stream error: {}", e)))?;

    let metadata = match first_msg.data {
        Some(crate::grpc::proto::upload_document_request::Data::Metadata(m)) => m,
        _ => {
            return Err(Status::invalid_argument(
                "First message must contain metadata",
            ))
        }
    };

    let filename = if metadata.filename.is_empty() {
        "unnamed".to_string()
    } else {
        metadata.filename
    };

    let mime_type = if metadata.mime_type.is_empty() {
        "application/octet-stream".to_string()
    } else {
        metadata.mime_type
    };

    // Collect file data from subsequent chunks
    let mut file_data = Vec::new();
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| Status::internal(format!("Stream error: {}", e)))?;
        if let Some(crate::grpc::proto::upload_document_request::Data::Chunk(chunk)) = msg.data {
            file_data.extend_from_slice(&chunk);

            // Check size limit (20MB)
            if file_data.len() > 20 * 1024 * 1024 {
                return Err(Status::invalid_argument("File too large (max 20MB)"));
            }
        }
    }

    let size = file_data.len() as i64;

    // Generate storage key
    let extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    let storage_key = format!("{}/{}.{}", Uuid::new_v4(), Uuid::new_v4(), extension);

    // Clone tenant_id for metrics before moving into Document
    let tenant_id_for_metrics = tenant.app_id.clone();

    // Create document
    let mut document = Document::new(
        tenant.app_id,
        tenant.org_id,
        tenant.user_id,
        filename,
        mime_type,
        size,
        storage_key.clone(),
    );

    tracing::info!(
        document_id = %document.id,
        filename = %document.original_name,
        size = %size,
        "Document upload started via gRPC"
    );

    // Upload to storage
    state
        .storage
        .upload(&storage_key, file_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload file to storage: {}", e);
            Status::internal(format!("Storage error: {}", e))
        })?;

    // Set status to Ready and save to DB
    document.status = ModelDocumentStatus::Ready;

    state
        .db
        .documents()
        .insert_one(&document, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert document: {}", e);
            Status::internal(format!("Database error: {}", e))
        })?;

    // Record metering metrics
    let labels = [
        ("tenant_id", tenant_id_for_metrics.clone()),
        ("mime_type", document.mime_type.clone()),
    ];
    counter!("document_uploads_total", &labels).increment(1);
    histogram!(
        "document_upload_bytes",
        &[("tenant_id", tenant_id_for_metrics)]
    )
    .record(size as f64);

    tracing::info!(document_id = %document.id, "Document upload completed via gRPC");

    Ok(Response::new(UploadDocumentResponse {
        document: Some(document_to_proto(&document)),
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn download_document(
    state: &AppState,
    request: Request<DownloadDocumentRequest>,
) -> Result<Response<DownloadStream>, Status> {
    let req = request.get_ref();

    // Capability check only if not using signed URL
    let has_signature = req.signature.is_some() && req.expires.is_some();
    if !has_signature {
        state
            .capability_checker
            .require_capability(&request, capabilities::DOCUMENT_DOWNLOAD)
            .await?;
    }

    // Check for signed URL parameters
    let is_signed = if let (Some(signature), Some(expires)) = (&req.signature, &req.expires) {
        service_core::utils::signature::validate_document_signature(
            &req.document_id,
            signature,
            *expires,
            &state.config.signature.signing_secret,
        )
        .map_err(|e| Status::permission_denied(format!("Invalid signature: {}", e)))?;
        true
    } else {
        false
    };

    // Get tenant context if not signed
    let tenant = if !is_signed {
        Some(extract_tenant_context(&request)?)
    } else {
        None
    };

    // Extract tenant_id for metrics before tenant is moved
    let tenant_id_for_metrics = tenant
        .as_ref()
        .map(|t| t.app_id.clone())
        .unwrap_or_else(|| "signed_url".to_string());

    // Fetch document
    let document = if is_signed {
        state
            .db
            .documents()
            .find_one(doc! { "_id": &req.document_id }, None)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found("Document not found"))?
    } else {
        let tenant = tenant.unwrap();
        state
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
            .ok_or_else(|| Status::not_found("Document not found"))?
    };

    // Check for chunked video
    if let Some(ref metadata) = document.processing_metadata {
        if document.mime_type.starts_with("video/") {
            if let Some(ref chunks) = metadata.chunks {
                // Return chunked video info
                let chunked_info = ChunkedVideoInfo {
                    original_name: document.original_name.clone(),
                    resolution: metadata.resolution.clone(),
                    total_size: metadata.total_size.unwrap_or(0),
                    chunk_count: chunks.len() as i32,
                    chunks: chunks
                        .iter()
                        .map(|c| VideoChunkInfo {
                            index: c.index as i32,
                            size: c.size,
                        })
                        .collect(),
                };

                let response = DownloadDocumentResponse {
                    data: Some(
                        crate::grpc::proto::download_document_response::Data::ChunkedVideo(
                            chunked_info,
                        ),
                    ),
                };

                let stream = futures::stream::once(async move { Ok(response) });
                return Ok(Response::new(Box::pin(stream)));
            }
        }
    }

    // Determine storage key and content type
    let (storage_key, content_type, filename) =
        if let Some(ref metadata) = document.processing_metadata {
            if let Some(ref processed_path) = metadata.thumbnail_path {
                let ct = if processed_path.ends_with(".webp") {
                    "image/webp"
                } else if processed_path.ends_with(".mp4") {
                    "video/mp4"
                } else {
                    "application/octet-stream"
                };
                (
                    processed_path.clone(),
                    ct.to_string(),
                    document.original_name.clone(),
                )
            } else {
                (
                    document.storage_key.clone(),
                    document.mime_type.clone(),
                    document.original_name.clone(),
                )
            }
        } else {
            (
                document.storage_key.clone(),
                document.mime_type.clone(),
                document.original_name.clone(),
            )
        };

    // Download file
    let file_data = state.storage.download(&storage_key).await.map_err(|e| {
        tracing::error!("Failed to download file: {}", e);
        Status::internal(format!("Storage error: {}", e))
    })?;

    let total_size = file_data.len() as i64;

    // Record metering metrics
    let labels = [("tenant_id", tenant_id_for_metrics.clone())];
    counter!("document_downloads_total", &labels).increment(1);
    histogram!(
        "document_download_bytes",
        &[("tenant_id", tenant_id_for_metrics)]
    )
    .record(total_size as f64);

    // Create streaming response
    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        // Send metadata first
        let metadata_msg = DownloadDocumentResponse {
            data: Some(
                crate::grpc::proto::download_document_response::Data::Metadata(DownloadMetadata {
                    filename,
                    content_type,
                    size: total_size,
                }),
            ),
        };

        if tx.send(Ok(metadata_msg)).await.is_err() {
            return;
        }

        // Send file chunks
        for chunk in file_data.chunks(CHUNK_SIZE) {
            let chunk_msg = DownloadDocumentResponse {
                data: Some(crate::grpc::proto::download_document_response::Data::Chunk(
                    chunk.to_vec(),
                )),
            };

            if tx.send(Ok(chunk_msg)).await.is_err() {
                return;
            }
        }
    });

    Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
}

#[tracing::instrument(skip(state, request))]
pub async fn get_document(
    state: &AppState,
    request: Request<GetDocumentRequest>,
) -> Result<Response<GetDocumentResponse>, Status> {
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

    Ok(Response::new(GetDocumentResponse {
        document: Some(document_to_proto(&document)),
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn list_documents(
    state: &AppState,
    request: Request<ListDocumentsRequest>,
) -> Result<Response<ListDocumentsResponse>, Status> {
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_READ)
        .await?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let page = req.page.unwrap_or(1).max(1) as u64;
    let page_size = req.page_size.unwrap_or(20).clamp(1, 100) as u64;
    let skip = (page - 1) * page_size;

    let mut filter = doc! {
        "app_id": &tenant.app_id,
        "org_id": &tenant.org_id,
        "owner_id": &tenant.user_id
    };

    if let Some(status) = req.status {
        if let Some(model_status) = proto_to_status(status) {
            let bson_status = mongodb::bson::to_bson(&model_status)
                .map_err(|e| Status::internal(format!("Serialization error: {}", e)))?;
            filter.insert("status", bson_status);
        }
    }

    if let Some(mime_type) = req.mime_type {
        filter.insert("mime_type", doc! { "$regex": format!("^{}", mime_type) });
    }

    let total = state
        .db
        .documents()
        .count_documents(filter.clone(), None)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    let find_options = FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .skip(skip)
        .limit(page_size as i64)
        .build();

    let mut cursor = state
        .db
        .documents()
        .find(filter, find_options)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    let mut documents = Vec::new();
    while let Some(doc) = cursor
        .try_next()
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
    {
        documents.push(document_to_proto(&doc));
    }

    let total_pages = (total as f64 / page_size as f64).ceil() as i32;

    Ok(Response::new(ListDocumentsResponse {
        documents,
        total: total as i64,
        page: page as i32,
        page_size: page_size as i32,
        total_pages,
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn delete_document(
    state: &AppState,
    request: Request<DeleteDocumentRequest>,
) -> Result<Response<DeleteDocumentResponse>, Status> {
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_DELETE)
        .await?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Find and delete document
    let document = state
        .db
        .documents()
        .find_one_and_delete(
            doc! {
                "_id": &req.document_id,
                "app_id": &tenant.app_id,
                "org_id": &tenant.org_id
            },
            None,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    if let Some(doc) = document {
        // Delete from storage
        if let Err(e) = state.storage.delete(&doc.storage_key).await {
            tracing::warn!(
                document_id = %req.document_id,
                storage_key = %doc.storage_key,
                error = %e,
                "Failed to delete file from storage"
            );
        }

        // Delete processed files if any
        if let Some(metadata) = doc.processing_metadata {
            if let Some(thumbnail_path) = metadata.thumbnail_path {
                let _ = state.storage.delete(&thumbnail_path).await;
            }
            if let Some(chunks) = metadata.chunks {
                for chunk in chunks {
                    let _ = state.storage.delete(&chunk.path).await;
                }
            }
        }

        // Record metering metrics
        let labels = [("tenant_id", tenant.app_id.clone())];
        counter!("document_deletes_total", &labels).increment(1);

        tracing::info!(document_id = %req.document_id, "Document deleted");
        Ok(Response::new(DeleteDocumentResponse { success: true }))
    } else {
        Err(Status::not_found("Document not found"))
    }
}
