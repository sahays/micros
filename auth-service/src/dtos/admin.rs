use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::models::ClientType;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateClientRequest {
    #[validate(length(min = 1, message = "App name is required"))]
    #[schema(example = "My BFF App")]
    pub app_name: String,

    pub app_type: ClientType,

    #[validate(range(min = 1, message = "Rate limit must be at least 1"))]
    #[schema(example = 100)]
    pub rate_limit_per_min: u32,

    #[schema(example = "[\"http://localhost:3000\"]")]
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateClientResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub client_id: String,
    #[schema(example = "client-secret-123")]
    pub client_secret: String,
    #[schema(example = "signing-secret-key")]
    pub signing_secret: String,
    #[schema(example = "My BFF App")]
    pub app_name: String,
    pub app_type: ClientType,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateSecretResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub client_id: String,
    #[schema(example = "new-client-secret-456")]
    pub new_client_secret: String,
    #[schema(example = "new-signing-secret-key")]
    pub new_signing_secret: String,
    #[schema(value_type = String, format = "date-time")]
    pub previous_secret_expiry: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateServiceAccountRequest {
    #[validate(length(min = 1, message = "Service name is required"))]
    #[schema(example = "Payments Service")]
    pub service_name: String,
    #[schema(example = "[\"read:payments\", \"write:payments\"]")]
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateServiceAccountResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub service_id: String,
    #[schema(example = "svc_live_random_part_123")]
    pub api_key: String,
    #[schema(example = "Payments Service")]
    pub service_name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RotateServiceKeyResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub service_id: String,
    #[schema(example = "svc_live_new_random_part_456")]
    pub new_api_key: String,
    #[schema(value_type = String, format = "date-time")]
    pub previous_key_expiry: chrono::DateTime<chrono::Utc>,
}
