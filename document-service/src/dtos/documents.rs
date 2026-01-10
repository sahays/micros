use crate::models::DocumentStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub id: String,
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub s3_key: String,
    pub status: DocumentStatus,
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
            s3_key: doc.s3_key,
            status: doc.status,
            created_at: doc.created_at.to_rfc3339(),
            updated_at: doc.updated_at.to_rfc3339(),
        }
    }
}
