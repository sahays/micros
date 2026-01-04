use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceAccount {
    #[serde(rename = "_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub service_id: String,
    #[schema(read_only)]
    pub api_key_hash: String, // Argon2 hash for verification
    #[schema(read_only)]
    pub api_key_lookup_hash: String, // SHA-256 hex for lookup
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub previous_api_key_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub previous_api_key_lookup_hash: Option<String>,
    #[serde(
        default,
        with = "crate::models::client::optional_chrono_datetime_as_bson_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = "date-time")]
    pub previous_key_expiry: Option<DateTime<Utc>>,
    #[schema(example = "Payments Service")]
    pub service_name: String,
    pub scopes: Vec<String>,
    pub enabled: bool,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: DateTime<Utc>,
    #[serde(
        default,
        with = "crate::models::client::optional_chrono_datetime_as_bson_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = "date-time")]
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ServiceAccount {
    pub fn new(
        service_name: String,
        api_key_hash: String,
        api_key_lookup_hash: String,
        scopes: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let service_id = Uuid::new_v4().to_string();
        Self {
            id: service_id.clone(),
            service_id,
            api_key_hash,
            api_key_lookup_hash,
            previous_api_key_hash: None,
            previous_api_key_lookup_hash: None,
            previous_key_expiry: None,
            service_name,
            scopes,
            enabled: true,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    pub fn calculate_lookup_hash(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hex::encode(hasher.finalize())
    }
}
