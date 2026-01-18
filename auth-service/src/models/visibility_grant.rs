//! Visibility grant model - cross-subtree visibility permissions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Visibility grant entity - allows user to see nodes outside their assigned subtree.
#[derive(Debug, Clone, FromRow)]
pub struct VisibilityGrant {
    pub grant_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub created_utc: DateTime<Utc>,
}

impl VisibilityGrant {
    /// Create a new visibility grant.
    pub fn new(tenant_id: Uuid, user_id: Uuid, org_node_id: Uuid) -> Self {
        Self {
            grant_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            org_node_id,
            created_utc: Utc::now(),
        }
    }
}

/// Request to create a visibility grant.
#[derive(Debug, Deserialize)]
pub struct CreateVisibilityGrantRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
}

/// Visibility grant response for API.
#[derive(Debug, Serialize)]
pub struct VisibilityGrantResponse {
    pub grant_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub created_utc: DateTime<Utc>,
}

impl From<VisibilityGrant> for VisibilityGrantResponse {
    fn from(g: VisibilityGrant) -> Self {
        Self {
            grant_id: g.grant_id,
            tenant_id: g.tenant_id,
            user_id: g.user_id,
            org_node_id: g.org_node_id,
            created_utc: g.created_utc,
        }
    }
}
