use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    #[serde(rename = "_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,

    /// Application ID (maps to Client.client_id) - identifies which app this user belongs to
    #[schema(example = "770e8400-e29b-41d4-a716-446655440002")]
    pub app_id: String,

    /// Organization ID - identifies which org within the app this user belongs to
    #[schema(example = "660e8400-e29b-41d4-a716-446655440001")]
    pub org_id: String,

    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(read_only)]
    pub password_hash: String,
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    pub verified: bool,
    pub google_id: Option<String>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NewUser {
    pub app_id: String,
    pub org_id: String,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
}

impl User {
    pub fn new(
        app_id: String,
        org_id: String,
        email: String,
        password_hash: String,
        name: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            app_id,
            org_id,
            email,
            password_hash,
            name,
            verified: false,
            google_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn sanitized(&self) -> SanitizedUser {
        SanitizedUser {
            id: self.id.clone(),
            app_id: self.app_id.clone(),
            org_id: self.org_id.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
            verified: self.verified,
            google_id: self.google_id.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// User without sensitive fields (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SanitizedUser {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    #[schema(example = "770e8400-e29b-41d4-a716-446655440002")]
    pub app_id: String,
    #[schema(example = "660e8400-e29b-41d4-a716-446655440001")]
    pub org_id: String,
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    pub verified: bool,
    pub google_id: Option<String>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: DateTime<Utc>,
}
