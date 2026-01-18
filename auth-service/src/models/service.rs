//! Service model - Know-Your-Service (KYS) registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Service state codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Active,
    Disabled,
}

impl ServiceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceState::Active => "active",
            ServiceState::Disabled => "disabled",
        }
    }
}

/// Service entity (BFF/domain service registration).
#[derive(Debug, Clone, FromRow)]
pub struct Service {
    pub svc_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub svc_key: String,
    pub svc_label: String,
    pub svc_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl Service {
    /// Create a new service.
    pub fn new(tenant_id: Option<Uuid>, svc_key: String, svc_label: String) -> Self {
        Self {
            svc_id: Uuid::new_v4(),
            tenant_id,
            svc_key,
            svc_label,
            svc_state_code: ServiceState::Active.as_str().to_string(),
            created_utc: Utc::now(),
        }
    }

    /// Check if service is active.
    pub fn is_active(&self) -> bool {
        self.svc_state_code == ServiceState::Active.as_str()
    }
}

/// Service secret entity.
#[derive(Debug, Clone, FromRow)]
pub struct ServiceSecret {
    pub secret_id: Uuid,
    pub svc_id: Uuid,
    pub secret_hash_text: String,
    pub created_utc: DateTime<Utc>,
    pub revoked_utc: Option<DateTime<Utc>>,
}

impl ServiceSecret {
    /// Create a new service secret.
    pub fn new(svc_id: Uuid, secret_hash: String) -> Self {
        Self {
            secret_id: Uuid::new_v4(),
            svc_id,
            secret_hash_text: secret_hash,
            created_utc: Utc::now(),
            revoked_utc: None,
        }
    }

    /// Check if secret is valid (not revoked).
    pub fn is_valid(&self) -> bool {
        self.revoked_utc.is_none()
    }
}

/// Service permission entity.
#[derive(Debug, Clone, FromRow)]
pub struct ServicePermission {
    pub svc_id: Uuid,
    pub perm_key: String,
}

/// Service session entity (for token-based auth).
#[derive(Debug, Clone, FromRow)]
pub struct ServiceSession {
    pub svc_session_id: Uuid,
    pub svc_id: Uuid,
    pub token_hash_text: String,
    pub expiry_utc: DateTime<Utc>,
    pub revoked_utc: Option<DateTime<Utc>>,
    pub created_utc: DateTime<Utc>,
}

/// Request to register a service.
#[derive(Debug, Deserialize)]
pub struct RegisterServiceRequest {
    pub tenant_id: Option<Uuid>,
    pub svc_key: String,
    pub svc_label: String,
    pub permissions: Option<Vec<String>>,
}

/// Service registration response (includes plaintext secret once).
#[derive(Debug, Serialize)]
pub struct RegisterServiceResponse {
    pub svc_id: Uuid,
    pub svc_key: String,
    pub svc_secret: String, // Plaintext, returned only once
}

/// Service response for API.
#[derive(Debug, Serialize)]
pub struct ServiceResponse {
    pub svc_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub svc_key: String,
    pub svc_label: String,
    pub svc_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Service> for ServiceResponse {
    fn from(s: Service) -> Self {
        Self {
            svc_id: s.svc_id,
            tenant_id: s.tenant_id,
            svc_key: s.svc_key,
            svc_label: s.svc_label,
            svc_state_code: s.svc_state_code,
            created_utc: s.created_utc,
        }
    }
}

/// Secret rotation response.
#[derive(Debug, Serialize)]
pub struct RotateSecretResponse {
    pub svc_secret: String, // Plaintext, returned only once
}

/// Service token response.
#[derive(Debug, Serialize)]
pub struct ServiceTokenResponse {
    pub service_token: String,
    pub expires_in: i64,
}

/// Service context extracted from request.
#[derive(Debug, Clone)]
pub struct ServiceContext {
    pub svc_id: Uuid,
    pub svc_key: String,
    pub tenant_id: Option<Uuid>,
    pub permissions: Vec<String>,
}

impl ServiceContext {
    /// Check if service has a specific permission.
    pub fn has_permission(&self, perm_key: &str) -> bool {
        self.permissions.iter().any(|p| p == perm_key)
    }
}
