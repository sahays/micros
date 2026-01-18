//! Org assignment model - time-bounded user→org→role assignments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Org assignment entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrgAssignment {
    pub assignment_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
}

impl OrgAssignment {
    /// Create a new assignment starting now.
    pub fn new(tenant_id: Uuid, user_id: Uuid, org_node_id: Uuid, role_id: Uuid) -> Self {
        Self {
            assignment_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            org_node_id,
            role_id,
            start_utc: Utc::now(),
            end_utc: None,
        }
    }

    /// Create a new assignment with specific start time.
    pub fn new_with_start(
        tenant_id: Uuid,
        user_id: Uuid,
        org_node_id: Uuid,
        role_id: Uuid,
        start_utc: DateTime<Utc>,
    ) -> Self {
        Self {
            assignment_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            org_node_id,
            role_id,
            start_utc,
            end_utc: None,
        }
    }

    /// Check if assignment is currently active.
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        self.start_utc <= now && self.end_utc.is_none_or(|end| end > now)
    }

    /// Check if assignment has ended.
    pub fn has_ended(&self) -> bool {
        self.end_utc.is_some_and(|end| end <= Utc::now())
    }
}

/// Request to create an assignment.
#[derive(Debug, Deserialize)]
pub struct CreateAssignmentRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub start_utc: Option<DateTime<Utc>>,
}

/// Assignment response for API.
#[derive(Debug, Serialize)]
pub struct AssignmentResponse {
    pub assignment_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl From<OrgAssignment> for AssignmentResponse {
    fn from(a: OrgAssignment) -> Self {
        let is_active = a.is_active();
        Self {
            assignment_id: a.assignment_id,
            tenant_id: a.tenant_id,
            user_id: a.user_id,
            org_node_id: a.org_node_id,
            role_id: a.role_id,
            start_utc: a.start_utc,
            end_utc: a.end_utc,
            is_active,
        }
    }
}

/// Assignment with role and org details.
#[derive(Debug, Serialize, FromRow)]
pub struct AssignmentDetail {
    pub assignment_id: Uuid,
    pub org_node_id: Uuid,
    pub org_node_label: String,
    pub role_id: Uuid,
    pub role_label: String,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
}
