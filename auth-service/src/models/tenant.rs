//! Tenant model - root of multi-tenancy hierarchy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Tenant state codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantState {
    Active,
    Suspended,
}

impl TenantState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TenantState::Active => "active",
            TenantState::Suspended => "suspended",
        }
    }
}

/// Tenant entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub tenant_id: Uuid,
    pub tenant_slug: String,
    pub tenant_label: String,
    pub tenant_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl Tenant {
    /// Create a new tenant.
    pub fn new(tenant_slug: String, tenant_label: String) -> Self {
        Self {
            tenant_id: Uuid::new_v4(),
            tenant_slug,
            tenant_label,
            tenant_state_code: TenantState::Active.as_str().to_string(),
            created_utc: Utc::now(),
        }
    }

    /// Check if tenant is active.
    pub fn is_active(&self) -> bool {
        self.tenant_state_code == TenantState::Active.as_str()
    }
}

/// Request to create a tenant.
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_slug: String,
    pub tenant_label: String,
}

/// Tenant response for API.
#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub tenant_id: Uuid,
    pub tenant_slug: String,
    pub tenant_label: String,
    pub tenant_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Tenant> for TenantResponse {
    fn from(t: Tenant) -> Self {
        Self {
            tenant_id: t.tenant_id,
            tenant_slug: t.tenant_slug,
            tenant_label: t.tenant_label,
            tenant_state_code: t.tenant_state_code,
            created_utc: t.created_utc,
        }
    }
}
