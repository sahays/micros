use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::models::{AuthPolicy, ClientType, OrgSettings, SanitizedOrganization};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateClientRequest {
    #[validate(length(min = 1, message = "App name is required"))]
    #[schema(example = "My BFF App")]
    pub app_name: String,

    pub app_type: ClientType,

    /// Rate limit per minute (0 = unlimited for service clients)
    #[schema(example = 0)]
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

// ============================================================================
// Organization Admin DTOs
// ============================================================================

/// Request to create a new organization under the calling app.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateOrganizationRequest {
    /// Display name for the organization
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    #[schema(example = "Acme Corporation")]
    pub name: String,

    /// Optional organization settings
    pub settings: Option<OrgSettings>,

    /// Optional custom auth policies (defaults used if not provided)
    pub auth_policy: Option<AuthPolicy>,
}

/// Response after creating an organization.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateOrganizationResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub org_id: String,
    #[schema(example = "770e8400-e29b-41d4-a716-446655440002")]
    pub app_id: String,
    #[schema(example = "Acme Corporation")]
    pub name: String,
    #[schema(value_type = String, format = "date-time")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request to update an organization.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateOrganizationRequest {
    /// New display name (optional)
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    #[schema(example = "Acme Corp Updated")]
    pub name: Option<String>,

    /// Updated settings (optional)
    pub settings: Option<OrgSettings>,

    /// Enable/disable the organization
    pub enabled: Option<bool>,
}

/// Response listing organizations for an app.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListOrganizationsResponse {
    pub organizations: Vec<SanitizedOrganization>,
    pub total: usize,
}

/// Request to update auth policies for an organization.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateAuthPolicyRequest {
    pub auth_policy: AuthPolicy,
}
