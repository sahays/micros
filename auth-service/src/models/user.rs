use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub verified: bool,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
}

impl User {
    pub fn new(email: String, password_hash: String, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            password_hash,
            name,
            verified: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn sanitized(&self) -> SanitizedUser {
        SanitizedUser {
            id: self.id.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
            verified: self.verified,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// User without sensitive fields (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub verified: bool,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
}
