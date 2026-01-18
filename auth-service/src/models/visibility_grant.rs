//! Visibility grant model - cross-subtree visibility permissions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Access scope codes for visibility grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessScope {
    Read,
    Analyze,
}

impl AccessScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessScope::Read => "read",
            AccessScope::Analyze => "analyze",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "analyze" => AccessScope::Analyze,
            _ => AccessScope::Read,
        }
    }
}

/// Visibility grant entity - allows user to see nodes outside their assigned subtree.
#[derive(Debug, Clone, FromRow)]
pub struct VisibilityGrant {
    pub grant_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub access_scope_code: String,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
}

impl VisibilityGrant {
    /// Create a new visibility grant.
    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        org_node_id: Uuid,
        access_scope: AccessScope,
        start_utc: Option<DateTime<Utc>>,
        end_utc: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            grant_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            org_node_id,
            access_scope_code: access_scope.as_str().to_string(),
            start_utc: start_utc.unwrap_or_else(Utc::now),
            end_utc,
        }
    }

    /// Check if grant is currently active.
    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        self.start_utc <= now && self.end_utc.is_none_or(|end| end > now)
    }

    /// Get access scope as enum.
    pub fn access_scope(&self) -> AccessScope {
        AccessScope::parse(&self.access_scope_code)
    }
}

/// Request to create a visibility grant.
#[derive(Debug, Deserialize)]
pub struct CreateVisibilityGrantRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub access_scope: AccessScope,
    pub start_utc: Option<DateTime<Utc>>,
    pub end_utc: Option<DateTime<Utc>>,
}

/// Visibility grant response for API.
#[derive(Debug, Serialize)]
pub struct VisibilityGrantResponse {
    pub grant_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub access_scope_code: String,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
}

impl From<VisibilityGrant> for VisibilityGrantResponse {
    fn from(g: VisibilityGrant) -> Self {
        Self {
            grant_id: g.grant_id,
            tenant_id: g.tenant_id,
            user_id: g.user_id,
            org_node_id: g.org_node_id,
            access_scope_code: g.access_scope_code,
            start_utc: g.start_utc,
            end_utc: g.end_utc,
        }
    }
}
