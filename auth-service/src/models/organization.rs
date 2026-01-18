use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Authentication policy configuration for an organization.
/// Defines password requirements, MFA settings, and session management.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthPolicy {
    /// Minimum password length (default: 8)
    #[schema(example = 8)]
    pub password_min_length: u8,

    /// Require at least one uppercase letter
    #[schema(example = true)]
    pub password_require_uppercase: bool,

    /// Require at least one numeric digit
    #[schema(example = true)]
    pub password_require_number: bool,

    /// Require at least one special character
    #[schema(example = false)]
    pub password_require_special: bool,

    /// Whether MFA is required for all users in this org
    #[schema(example = false)]
    pub mfa_required: bool,

    /// Allowed MFA methods: "totp", "email"
    #[schema(example = json!(["totp", "email"]))]
    pub mfa_allowed_methods: Vec<String>,

    /// Session timeout in minutes (default: 60)
    #[schema(example = 60)]
    pub session_timeout_minutes: u32,

    /// Maximum failed login attempts before lockout
    #[schema(example = 5)]
    pub max_failed_attempts: u8,

    /// Account lockout duration in minutes after max failed attempts
    #[schema(example = 15)]
    pub lockout_duration_minutes: u32,
}

impl Default for AuthPolicy {
    fn default() -> Self {
        Self {
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: false,
            mfa_required: false,
            mfa_allowed_methods: vec!["totp".to_string(), "email".to_string()],
            session_timeout_minutes: 60,
            max_failed_attempts: 5,
            lockout_duration_minutes: 15,
        }
    }
}

/// Organization-specific settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct OrgSettings {
    /// Timezone for the organization (IANA timezone)
    #[schema(example = "America/New_York")]
    pub timezone: Option<String>,

    /// Locale for the organization
    #[schema(example = "en-US")]
    pub locale: Option<String>,
}

/// An organization (tenant) within an application.
/// Organizations belong to an app (identified by app_id/client_id) and contain users.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Organization {
    /// MongoDB document ID (UUID string)
    #[serde(rename = "_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,

    /// Unique organization identifier
    #[schema(example = "660e8400-e29b-41d4-a716-446655440001")]
    pub org_id: String,

    /// Parent application ID (maps to Client.client_id)
    #[schema(example = "770e8400-e29b-41d4-a716-446655440002")]
    pub app_id: String,

    /// Display name of the organization
    #[schema(example = "Acme Corporation")]
    pub name: String,

    /// Organization-specific settings
    pub settings: OrgSettings,

    /// Authentication policies for this organization
    pub auth_policy: AuthPolicy,

    /// Whether this organization is active
    pub enabled: bool,

    /// Timestamp when the organization was created
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: DateTime<Utc>,

    /// Timestamp when the organization was last updated
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Create a new organization with default settings and policies.
    pub fn new(app_id: String, name: String) -> Self {
        let now = Utc::now();
        let org_id = Uuid::new_v4().to_string();
        Self {
            id: org_id.clone(),
            org_id,
            app_id,
            name,
            settings: OrgSettings::default(),
            auth_policy: AuthPolicy::default(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new organization with custom settings and policies.
    pub fn with_config(
        app_id: String,
        name: String,
        settings: OrgSettings,
        auth_policy: AuthPolicy,
    ) -> Self {
        let now = Utc::now();
        let org_id = Uuid::new_v4().to_string();
        Self {
            id: org_id.clone(),
            org_id,
            app_id,
            name,
            settings,
            auth_policy,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Sanitized organization for API responses (same as Organization for now,
/// but provides a consistent pattern if we add sensitive fields later).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SanitizedOrganization {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub org_id: String,

    #[schema(example = "770e8400-e29b-41d4-a716-446655440002")]
    pub app_id: String,

    #[schema(example = "Acme Corporation")]
    pub name: String,

    pub settings: OrgSettings,

    pub auth_policy: AuthPolicy,

    pub enabled: bool,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: DateTime<Utc>,

    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: DateTime<Utc>,
}

impl From<Organization> for SanitizedOrganization {
    fn from(org: Organization) -> Self {
        Self {
            org_id: org.org_id,
            app_id: org.app_id,
            name: org.name,
            settings: org.settings,
            auth_policy: org.auth_policy,
            enabled: org.enabled,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}
