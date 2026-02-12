use crate::grpc::capability_check::capabilities;
use crate::grpc::document_service::{extract_tenant_context, ChunkDownloadStream, CHUNK_SIZE};
use crate::grpc::proto::{
    ChunkDownloadMetadata, DownloadVideoChunkRequest, DownloadVideoChunkResponse,
};
use crate::startup::AppState;
use mongodb::bson::doc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn download_video_chunk(
    state: &AppState,
    request: Request<DownloadVideoChunkRequest>,
) -> Result<Response<ChunkDownloadStream>, Status> {
    // Capability check (if enabled)
    state
        .capability_checker
        .require_capability(&request, capabilities::DOCUMENT_DOWNLOAD)
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
    let chunk_data = state
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
                    crate::grpc::proto::download_video_chunk_response::Data::Chunk(chunk.to_vec()),
                ),
            };

            if tx.send(Ok(chunk_msg)).await.is_err() {
                return;
            }
        }
    });

    Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
}
