use crate::grpc::document_service::{
    document_to_proto, extract_tenant_context, proto_to_status,
};
use crate::grpc::proto::{
    DeleteDocumentRequest, DeleteDocumentResponse, DownloadDocumentRequest,
    DownloadDocumentResponse, GetDocumentRequest, GetDocumentResponse, ListDocumentsRequest,
    ListDocumentsResponse, UploadDocumentRequest, UploadDocumentResponse,
};
use crate::models::{Document, DocumentStatus as ModelDocumentStatus};
use crate::startup::AppState;
use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[tracing::instrument(skip(state, request))]
pub async fn upload_document(
    state: &AppState,
    request: Request<UploadDocumentRequest>,
) -> Result<Response<UploadDocumentResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let filename = if req.filename.is_empty() {
        "unnamed".to_string()
    } else {
        req.filename
    };

    let mime_type = if req.mime_type.is_empty() {
        "application/octet-stream".to_string()
    } else {
        req.mime_type
    };

    let file_data = req.data;

    // Check size limit (20MB)
    if file_data.len() > 20 * 1024 * 1024 {
        return Err(Status::invalid_argument("File too large (max 20MB)"));
    }

    let size = file_data.len() as i64;

    // Generate storage key: {app_id}/{tenant_id}/{document_id}.{ext}
    let extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    let document_id = Uuid::new_v4().to_string();
    let storage_key = format!("{}/{}/{}.{}", tenant.app_id, tenant.tenant_id, document_id, extension);

    // Create document
    let mut document = Document::new(
        tenant.app_id,
        tenant.tenant_id,
        tenant.user_id,
        filename,
        mime_type,
        size,
        storage_key.clone(),
    );
    // Use the pre-generated document_id
    document.id = document_id;

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

    tracing::info!(document_id = %document.id, "Document upload completed via gRPC");

    Ok(Response::new(UploadDocumentResponse {
        document: Some(document_to_proto(&document)),
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn download_document(
    state: &AppState,
    request: Request<DownloadDocumentRequest>,
) -> Result<Response<DownloadDocumentResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Fetch document with tenant filter
    let document = state
        .db
        .documents()
        .find_one(
            doc! {
                "_id": &req.document_id,
                "app_id": &tenant.app_id,
                "tenant_id": &tenant.tenant_id
            },
            None,
        )
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found("Document not found"))?;

    // Download file
    let file_data = state
        .storage
        .download(&document.storage_key)
        .await
        .map_err(|e| {
            tracing::error!("Failed to download file: {}", e);
            Status::internal(format!("Storage error: {}", e))
        })?;

    let total_size = file_data.len() as i64;

    Ok(Response::new(DownloadDocumentResponse {
        filename: document.original_name,
        content_type: document.mime_type,
        size: total_size,
        data: file_data,
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn get_document(
    state: &AppState,
    request: Request<GetDocumentRequest>,
) -> Result<Response<GetDocumentResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let document = state
        .db
        .documents()
        .find_one(
            doc! {
                "_id": &req.document_id,
                "app_id": &tenant.app_id,
                "tenant_id": &tenant.tenant_id
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
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let page = req.page.unwrap_or(1).max(1) as u64;
    let page_size = req.page_size.unwrap_or(20).clamp(1, 100) as u64;
    let skip = (page - 1) * page_size;

    let mut filter = doc! {
        "app_id": &tenant.app_id,
        "tenant_id": &tenant.tenant_id,
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
                "tenant_id": &tenant.tenant_id
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

        tracing::info!(document_id = %req.document_id, "Document deleted");
        Ok(Response::new(DeleteDocumentResponse { success: true }))
    } else {
        Err(Status::not_found("Document not found"))
    }
}
