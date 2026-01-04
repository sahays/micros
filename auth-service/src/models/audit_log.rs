use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub timestamp: DateTime<Utc>,
    #[schema(example = "service_auth")]
    pub event_type: String,
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub service_id: Option<String>,
    #[schema(example = "Payments Service")]
    pub service_name: Option<String>,
    #[schema(example = "/auth/app/token")]
    pub endpoint: String,
    #[schema(example = "POST")]
    pub method: String,
    #[schema(example = 200)]
    pub status_code: u16,
    #[schema(example = "127.0.0.1")]
    pub ip_address: String,
    pub details: Option<String>,
    pub scopes: Option<Vec<String>>,
}

impl AuditLog {
    pub fn new(
        event_type: String,
        service_id: Option<String>,
        endpoint: String,
        method: String,
        status_code: u16,
        ip_address: String,
    ) -> Self {
        Self {
            id: None,
            timestamp: Utc::now(),
            event_type,
            service_id,
            service_name: None,
            endpoint,
            method,
            status_code,
            ip_address,
            details: None,
            scopes: None,
        }
    }
}
