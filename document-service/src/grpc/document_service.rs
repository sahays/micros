use crate::dtos::{
    ImageOptions, PdfOptions, ProcessingOptions, ProcessorType as DtoProcessorType, VideoOptions,
};
use crate::grpc::proto::{
    document_service_server::DocumentService, ChunkDownloadMetadata, ChunkedVideoInfo,
    DeleteDocumentRequest, DeleteDocumentResponse, Document as ProtoDocument, DocumentStatus,
    DownloadDocumentRequest, DownloadDocumentResponse, DownloadMetadata, DownloadVideoChunkRequest,
    DownloadVideoChunkResponse, GenerateSignedUrlRequest, GenerateSignedUrlResponse,
    GetDocumentRequest, GetDocumentResponse, GetProcessingStatusRequest,
    GetProcessingStatusResponse, ListDocumentsRequest, ListDocumentsResponse,
    ProcessDocumentRequest, ProcessDocumentResponse, ProcessingMetadata as ProtoProcessingMetadata,
    ProcessingOptions as ProtoProcessingOptions, ProcessingProgress, ProcessorType,
    UploadDocumentRequest, UploadDocumentResponse, VideoChunkInfo,
};
use crate::middleware::tenant::TenantContext;
use crate::models::{Document, DocumentStatus as ModelDocumentStatus};
use crate::startup::AppState;
use crate::workers::ProcessingJob;
use futures::stream::TryStreamExt;
use futures::StreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use prost_types::Timestamp;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks for streaming

pub struct DocumentGrpcService {
    state: AppState,
}

impl DocumentGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Extract tenant context from gRPC metadata.
    fn extract_tenant_context(
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

    /// Extract tenant context from streaming request metadata.
    fn extract_tenant_context_from_streaming<T>(
        request: &Request<Streaming<T>>,
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
}

// Conversion helpers
fn status_to_proto(status: &ModelDocumentStatus) -> i32 {
    match status {
        ModelDocumentStatus::Uploading => DocumentStatus::Uploading as i32,
        ModelDocumentStatus::Processing => DocumentStatus::Processing as i32,
        ModelDocumentStatus::Ready => DocumentStatus::Ready as i32,
        ModelDocumentStatus::Failed => DocumentStatus::Failed as i32,
    }
}

fn proto_to_status(status: i32) -> Option<ModelDocumentStatus> {
    match DocumentStatus::try_from(status) {
        Ok(DocumentStatus::Uploading) => Some(ModelDocumentStatus::Uploading),
        Ok(DocumentStatus::Processing) => Some(ModelDocumentStatus::Processing),
        Ok(DocumentStatus::Ready) => Some(ModelDocumentStatus::Ready),
        Ok(DocumentStatus::Failed) => Some(ModelDocumentStatus::Failed),
        _ => None,
    }
}

fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn document_to_proto(doc: &Document) -> ProtoDocument {
    ProtoDocument {
        id: doc.id.clone(),
        app_id: doc.app_id.clone(),
        org_id: doc.org_id.clone(),
        owner_id: doc.owner_id.clone(),
        original_name: doc.original_name.clone(),
        mime_type: doc.mime_type.clone(),
        size: doc.size,
        storage_key: doc.storage_key.clone(),
        status: status_to_proto(&doc.status),
        error_message: doc.error_message.clone(),
        processing_metadata: doc
            .processing_metadata
            .as_ref()
            .map(|m| ProtoProcessingMetadata {
                extracted_text: m.extracted_text.clone(),
                page_count: m.page_count,
                duration_seconds: m.duration_seconds,
                optimized_size: m.optimized_size,
                thumbnail_path: m.thumbnail_path.clone(),
                error_details: m.error_details.clone(),
                resolution: m.resolution.clone(),
                chunk_count: m.chunk_count,
                total_size: m.total_size,
            }),
        created_at: Some(datetime_to_timestamp(doc.created_at)),
        updated_at: Some(datetime_to_timestamp(doc.updated_at)),
    }
}

fn proto_to_processing_options(opts: Option<ProtoProcessingOptions>) -> ProcessingOptions {
    match opts {
        Some(o) => ProcessingOptions {
            processors: if o.processors.is_empty() {
                None
            } else {
                Some(
                    o.processors
                        .iter()
                        .filter_map(|p| match ProcessorType::try_from(*p) {
                            Ok(ProcessorType::Pdf) => Some(DtoProcessorType::Pdf),
                            Ok(ProcessorType::Image) => Some(DtoProcessorType::Image),
                            Ok(ProcessorType::Video) => Some(DtoProcessorType::Video),
                            _ => None,
                        })
                        .collect(),
                )
            },
            pdf_options: o.pdf_options.map(|p| PdfOptions {
                extract_text: p.extract_text,
                extract_images: p.extract_images,
            }),
            image_options: o.image_options.map(|i| ImageOptions {
                format: if i.format.is_empty() {
                    "webp".to_string()
                } else {
                    i.format
                },
                quality: i.quality.clamp(1, 100) as u8,
            }),
            video_options: o.video_options.map(|v| VideoOptions {
                format: if v.format.is_empty() {
                    "hls".to_string()
                } else {
                    v.format
                },
                resolution: v.resolution,
            }),
        },
        None => ProcessingOptions::default(),
    }
}

type DownloadStream =
    Pin<Box<dyn futures::Stream<Item = Result<DownloadDocumentResponse, Status>> + Send>>;
type ChunkDownloadStream =
    Pin<Box<dyn futures::Stream<Item = Result<DownloadVideoChunkResponse, Status>> + Send>>;

#[tonic::async_trait]
impl DocumentService for DocumentGrpcService {
    type DownloadDocumentStream = DownloadStream;
    type DownloadVideoChunkStream = ChunkDownloadStream;

    #[tracing::instrument(skip(self, request))]
    async fn upload_document(
        &self,
        request: Request<Streaming<UploadDocumentRequest>>,
    ) -> Result<Response<UploadDocumentResponse>, Status> {
        let tenant = Self::extract_tenant_context_from_streaming(&request)?;
        let mut stream = request.into_inner();

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
            if let Some(crate::grpc::proto::upload_document_request::Data::Chunk(chunk)) = msg.data
            {
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
        self.state
            .storage
            .upload(&storage_key, file_data)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload file to storage: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?;

        // Set status to Ready and save to DB
        document.status = ModelDocumentStatus::Ready;

        self.state
            .db
            .documents()
            .insert_one(&document, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert document: {}", e);
                Status::internal(format!("Database error: {}", e))
            })?;

        tracing::info!(document_id = %document.id, "Document upload completed via gRPC");

        Ok(Response::new(UploadDocumentResponse {
            document: Some(document_to_proto(&document)),
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn download_document(
        &self,
        request: Request<DownloadDocumentRequest>,
    ) -> Result<Response<Self::DownloadDocumentStream>, Status> {
        let req = request.get_ref();

        // Check for signed URL parameters
        let is_signed = if let (Some(signature), Some(expires)) = (&req.signature, &req.expires) {
            service_core::utils::signature::validate_document_signature(
                &req.document_id,
                signature,
                *expires,
                &self.state.config.signature.signing_secret,
            )
            .map_err(|e| Status::permission_denied(format!("Invalid signature: {}", e)))?;
            true
        } else {
            false
        };

        // Get tenant context if not signed
        let tenant = if !is_signed {
            Some(Self::extract_tenant_context(&request)?)
        } else {
            None
        };

        // Fetch document
        let document = if is_signed {
            self.state
                .db
                .documents()
                .find_one(doc! { "_id": &req.document_id }, None)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?
                .ok_or_else(|| Status::not_found("Document not found"))?
        } else {
            let tenant = tenant.unwrap();
            self.state
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
        let file_data = self
            .state
            .storage
            .download(&storage_key)
            .await
            .map_err(|e| {
                tracing::error!("Failed to download file: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?;

        let total_size = file_data.len() as i64;

        // Create streaming response
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            // Send metadata first
            let metadata_msg = DownloadDocumentResponse {
                data: Some(
                    crate::grpc::proto::download_document_response::Data::Metadata(
                        DownloadMetadata {
                            filename,
                            content_type,
                            size: total_size,
                        },
                    ),
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

    #[tracing::instrument(skip(self, request))]
    async fn get_document(
        &self,
        request: Request<GetDocumentRequest>,
    ) -> Result<Response<GetDocumentResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        let document = self
            .state
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

    #[tracing::instrument(skip(self, request))]
    async fn list_documents(
        &self,
        request: Request<ListDocumentsRequest>,
    ) -> Result<Response<ListDocumentsResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
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

        let total = self
            .state
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

        let mut cursor = self
            .state
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

    #[tracing::instrument(skip(self, request))]
    async fn delete_document(
        &self,
        request: Request<DeleteDocumentRequest>,
    ) -> Result<Response<DeleteDocumentResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Find and delete document
        let document = self
            .state
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
            if let Err(e) = self.state.storage.delete(&doc.storage_key).await {
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
                    let _ = self.state.storage.delete(&thumbnail_path).await;
                }
                if let Some(chunks) = metadata.chunks {
                    for chunk in chunks {
                        let _ = self.state.storage.delete(&chunk.path).await;
                    }
                }
            }

            tracing::info!(document_id = %req.document_id, "Document deleted");
            Ok(Response::new(DeleteDocumentResponse { success: true }))
        } else {
            Err(Status::not_found("Document not found"))
        }
    }

    #[tracing::instrument(skip(self, request))]
    async fn process_document(
        &self,
        request: Request<ProcessDocumentRequest>,
    ) -> Result<Response<ProcessDocumentResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Fetch document
        let document = self
            .state
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
        self.state
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
        if let Some(job_tx) = &self.state.job_tx {
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

            tracing::info!(document_id = %document.id, "Processing job enqueued via gRPC");
        } else {
            return Err(Status::unavailable("Worker pool not available"));
        }

        Ok(Response::new(ProcessDocumentResponse {
            queued: true,
            status: DocumentStatus::Processing as i32,
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_processing_status(
        &self,
        request: Request<GetProcessingStatusRequest>,
    ) -> Result<Response<GetProcessingStatusResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        let document = self
            .state
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

    #[tracing::instrument(skip(self, request))]
    async fn generate_signed_url(
        &self,
        request: Request<GenerateSignedUrlRequest>,
    ) -> Result<Response<GenerateSignedUrlResponse>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Verify document exists and belongs to tenant
        let _document = self
            .state
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

        let expires_in = req.expires_in_seconds.max(60).min(86400); // 1 min to 24 hours
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
        let expires_timestamp = expires_at.timestamp();

        let signature = service_core::utils::signature::generate_document_signature(
            &req.document_id,
            expires_timestamp,
            &self.state.config.signature.signing_secret,
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

    #[tracing::instrument(skip(self, request))]
    async fn download_video_chunk(
        &self,
        request: Request<DownloadVideoChunkRequest>,
    ) -> Result<Response<Self::DownloadVideoChunkStream>, Status> {
        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Fetch document
        let document = self
            .state
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

        // Get chunks
        let chunks = document
            .processing_metadata
            .as_ref()
            .and_then(|m| m.chunks.as_ref())
            .ok_or_else(|| Status::not_found("Document is not chunked"))?;

        let chunk_index = req.chunk_index as usize;
        let chunk_info = chunks
            .get(chunk_index)
            .ok_or_else(|| Status::out_of_range("Chunk index out of range"))?;

        // Download chunk
        let chunk_data = self
            .state
            .storage
            .download(&chunk_info.path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to download chunk: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?;

        let chunk_size = chunk_data.len() as i64;

        // Create streaming response
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            // Send metadata first
            let metadata_msg = DownloadVideoChunkResponse {
                data: Some(
                    crate::grpc::proto::download_video_chunk_response::Data::Metadata(
                        ChunkDownloadMetadata {
                            index: chunk_index as i32,
                            size: chunk_size,
                            content_type: "video/mp4".to_string(),
                        },
                    ),
                ),
            };

            if tx.send(Ok(metadata_msg)).await.is_err() {
                return;
            }

            // Send chunk data
            for chunk in chunk_data.chunks(CHUNK_SIZE) {
                let chunk_msg = DownloadVideoChunkResponse {
                    data: Some(
                        crate::grpc::proto::download_video_chunk_response::Data::Chunk(
                            chunk.to_vec(),
                        ),
                    ),
                };

                if tx.send(Ok(chunk_msg)).await.is_err() {
                    return;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}
