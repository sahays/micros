use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentResponse {
    pub id: String,
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_key: String,
    pub status: String, // String here, mapped from backend enum
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}
