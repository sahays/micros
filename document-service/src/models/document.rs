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
pub struct ProcessingMetadata {
    pub extracted_text: Option<String>,
    pub page_count: Option<i32>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "_id")]
    pub id: String,
    /// Application ID (maps to registered client in auth-service)
    pub app_id: String,
    /// Organization ID within the application
    pub org_id: String,
    /// User who owns this document
    pub owner_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: i64,
    pub storage_key: String,
    pub status: DocumentStatus,
    pub error_message: Option<String>,
    pub processing_metadata: Option<ProcessingMetadata>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(
        app_id: String,
        org_id: String,
        owner_id: String,
        original_name: String,
        mime_type: String,
        size: i64,
        storage_key: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            app_id,
            org_id,
            owner_id,
            original_name,
            mime_type,
            size,
            storage_key,
            status: DocumentStatus::Uploading,
            error_message: None,
            processing_metadata: None,
            created_at: now,
            updated_at: now,
        }
    }
}
