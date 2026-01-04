use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub service_id: Option<String>,
    pub service_name: Option<String>,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
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
