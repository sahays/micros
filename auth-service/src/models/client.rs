use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    Web,
    Service,
    Mobile,
}

impl fmt::Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientType::Web => write!(f, "web"),
            ClientType::Service => write!(f, "service"),
            ClientType::Mobile => write!(f, "mobile"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub client_id: String,
    pub client_secret_hash: String,
    pub app_name: String,
    pub app_type: ClientType,
    pub rate_limit_per_min: u32,
    pub allowed_origins: Vec<String>,
    pub enabled: bool,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Client {
    pub fn new(
        client_id: String,
        client_secret_hash: String,
        app_name: String,
        app_type: ClientType,
        rate_limit_per_min: u32,
        allowed_origins: Vec<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: None,
            client_id,
            client_secret_hash,
            app_name,
            app_type,
            rate_limit_per_min,
            allowed_origins,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}
