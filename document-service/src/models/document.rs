use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Uploading,
    Processing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub index: usize,
    pub path: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetadata {
    pub extracted_text: Option<String>,
    pub page_count: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub optimized_size: Option<i64>,
    pub thumbnail_path: Option<String>,
    pub error_details: Option<String>,

    // Video chunking fields
    pub resolution: Option<String>,
    pub chunks: Option<Vec<ChunkInfo>>,
    pub chunk_count: Option<i32>,
    pub total_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "_id")]
    pub id: String,
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_key: String,
    pub status: DocumentStatus,
    pub error_message: Option<String>,
    pub processing_metadata: Option<ProcessingMetadata>,
    pub processing_attempts: i32,
    pub last_processing_attempt: Option<mongodb::bson::DateTime>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(
        owner_id: String,
        original_name: String,
        mime_type: String,
        size: i64,
        storage_key: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            owner_id,
            original_name,
            mime_type,
            size,
            storage_key,
            status: DocumentStatus::Uploading,
            error_message: None,
            processing_metadata: None,
            processing_attempts: 0,
            last_processing_attempt: None,
            created_at: now,
            updated_at: now,
        }
    }
}
