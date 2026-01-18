//! Org node model - hierarchical organization structure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Org node entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrgNode {
    pub org_node_id: Uuid,
    pub tenant_id: Uuid,
    pub node_type_code: String,
    pub node_label: String,
    pub parent_org_node_id: Option<Uuid>,
    pub active_flag: bool,
    pub created_utc: DateTime<Utc>,
}

impl OrgNode {
    /// Create a new org node.
    pub fn new(
        tenant_id: Uuid,
        node_type_code: String,
        node_label: String,
        parent_org_node_id: Option<Uuid>,
    ) -> Self {
        Self {
            org_node_id: Uuid::new_v4(),
            tenant_id,
            node_type_code,
            node_label,
            parent_org_node_id,
            active_flag: true,
            created_utc: Utc::now(),
        }
    }

    /// Check if this is a root node.
    pub fn is_root(&self) -> bool {
        self.parent_org_node_id.is_none()
    }
}

/// Org node path entry (closure table).
#[derive(Debug, Clone, FromRow)]
pub struct OrgNodePath {
    pub tenant_id: Uuid,
    pub ancestor_org_node_id: Uuid,
    pub descendant_org_node_id: Uuid,
    pub depth_val: i32,
}

/// Request to create an org node.
#[derive(Debug, Deserialize)]
pub struct CreateOrgNodeRequest {
    pub tenant_id: Uuid,
    pub node_type_code: String,
    pub node_label: String,
    pub parent_org_node_id: Option<Uuid>,
}

/// Request to update an org node.
#[derive(Debug, Deserialize)]
pub struct UpdateOrgNodeRequest {
    pub node_label: Option<String>,
    pub node_type_code: Option<String>,
}

/// Org node response for API.
#[derive(Debug, Serialize)]
pub struct OrgNodeResponse {
    pub org_node_id: Uuid,
    pub tenant_id: Uuid,
    pub node_type_code: String,
    pub node_label: String,
    pub parent_org_node_id: Option<Uuid>,
    pub active_flag: bool,
    pub created_utc: DateTime<Utc>,
}

impl From<OrgNode> for OrgNodeResponse {
    fn from(n: OrgNode) -> Self {
        Self {
            org_node_id: n.org_node_id,
            tenant_id: n.tenant_id,
            node_type_code: n.node_type_code,
            node_label: n.node_label,
            parent_org_node_id: n.parent_org_node_id,
            active_flag: n.active_flag,
            created_utc: n.created_utc,
        }
    }
}

/// Tree node with children for hierarchical response.
#[derive(Debug, Serialize)]
pub struct OrgTreeNode {
    #[serde(flatten)]
    pub node: OrgNodeResponse,
    pub children: Vec<OrgTreeNode>,
}
