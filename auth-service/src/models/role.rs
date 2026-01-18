//! Role model - tenant-scoped roles with capability mappings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Role entity (tenant-scoped).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub role_id: Uuid,
    pub tenant_id: Uuid,
    pub role_label: String,
    pub created_utc: DateTime<Utc>,
}

impl Role {
    /// Create a new role.
    pub fn new(tenant_id: Uuid, role_label: String) -> Self {
        Self {
            role_id: Uuid::new_v4(),
            tenant_id,
            role_label,
            created_utc: Utc::now(),
        }
    }
}

/// Role capability mapping.
#[derive(Debug, Clone, FromRow)]
pub struct RoleCapability {
    pub role_id: Uuid,
    pub cap_id: Uuid,
}

/// Request to create a role.
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub tenant_id: Uuid,
    pub role_label: String,
}

/// Request to assign capability to role.
#[derive(Debug, Deserialize)]
pub struct AssignCapabilityRequest {
    pub cap_id: Uuid,
}

/// Role response for API.
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub role_id: Uuid,
    pub tenant_id: Uuid,
    pub role_label: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Role> for RoleResponse {
    fn from(r: Role) -> Self {
        Self {
            role_id: r.role_id,
            tenant_id: r.tenant_id,
            role_label: r.role_label,
            created_utc: r.created_utc,
        }
    }
}

/// Role with capabilities for detailed response.
#[derive(Debug, Serialize)]
pub struct RoleWithCapabilities {
    #[serde(flatten)]
    pub role: RoleResponse,
    pub capabilities: Vec<String>,
}
