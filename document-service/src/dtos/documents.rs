use crate::models::{DocumentStatus, ProcessingMetadata};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_key: String,
    pub status: DocumentStatus,
    pub processing_metadata: Option<ProcessingMetadata>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::models::Document> for DocumentResponse {
    fn from(doc: crate::models::Document) -> Self {
        Self {
            id: doc.id,
            owner_id: doc.owner_id,
            original_name: doc.original_name,
            mime_type: doc.mime_type,
            size: doc.size,
            storage_key: doc.storage_key,
            status: doc.status,
            processing_metadata: doc.processing_metadata,
            created_at: doc.created_at.to_rfc3339(),
            updated_at: doc.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChunkedVideoResponse {
    #[serde(rename = "type")]
    pub type_: String,
    pub original_name: String,
    pub resolution: Option<String>,
    pub total_size: i64,
    pub chunk_count: usize,
    pub chunks: Vec<ChunkMetadata>,
}

#[derive(Debug, Serialize)]
pub struct ChunkMetadata {
    pub index: usize,
    pub url: String,
    pub size: i64,
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct DownloadParams {
    pub signature: Option<String>,
    pub expires: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentListParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<DocumentStatus>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}
