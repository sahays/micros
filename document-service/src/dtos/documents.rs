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
