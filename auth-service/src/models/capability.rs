//! Capability model - global capability registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Capability entity (global, not tenant-scoped).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Capability {
    pub cap_id: Uuid,
    pub cap_key: String,
    pub created_utc: DateTime<Utc>,
}

impl Capability {
    /// Create a new capability.
    pub fn new(cap_key: String) -> Self {
        Self {
            cap_id: Uuid::new_v4(),
            cap_key,
            created_utc: Utc::now(),
        }
    }

    /// Parse capability key into domain, resource, action, and scope.
    /// Format: {domain}.{resource}:{action}[:scope]
    /// Example: "crm.visit:view:subtree" -> ("crm", "visit", "view", Some("subtree"))
    pub fn parse_key(&self) -> Option<CapabilityParts> {
        let parts: Vec<&str> = self.cap_key.split(':').collect();
        if parts.is_empty() {
            return None;
        }

        let domain_resource: Vec<&str> = parts[0].split('.').collect();
        if domain_resource.len() != 2 {
            return None;
        }

        Some(CapabilityParts {
            domain: domain_resource[0].to_string(),
            resource: domain_resource[1].to_string(),
            action: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
            scope: parts.get(2).map(|s| s.to_string()),
        })
    }
}

/// Parsed capability parts.
#[derive(Debug, Clone)]
pub struct CapabilityParts {
    pub domain: String,
    pub resource: String,
    pub action: String,
    pub scope: Option<String>,
}

impl CapabilityParts {
    /// Check if this capability has "own" scope.
    pub fn is_own_scope(&self) -> bool {
        self.scope.as_deref() == Some("own")
    }

    /// Check if this capability has "subtree" scope.
    pub fn is_subtree_scope(&self) -> bool {
        self.scope.as_deref() == Some("subtree")
    }
}

/// Request to create a capability.
#[derive(Debug, Deserialize)]
pub struct CreateCapabilityRequest {
    pub cap_key: String,
}

/// Capability response for API.
#[derive(Debug, Serialize)]
pub struct CapabilityResponse {
    pub cap_id: Uuid,
    pub cap_key: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Capability> for CapabilityResponse {
    fn from(c: Capability) -> Self {
        Self {
            cap_id: c.cap_id,
            cap_key: c.cap_key,
            created_utc: c.created_utc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capability_key() {
        let cap = Capability::new("crm.visit:view:subtree".to_string());
        let parts = cap.parse_key().unwrap();
        assert_eq!(parts.domain, "crm");
        assert_eq!(parts.resource, "visit");
        assert_eq!(parts.action, "view");
        assert_eq!(parts.scope, Some("subtree".to_string()));
        assert!(parts.is_subtree_scope());
        assert!(!parts.is_own_scope());
    }

    #[test]
    fn test_parse_capability_key_no_scope() {
        let cap = Capability::new("org.node:create".to_string());
        let parts = cap.parse_key().unwrap();
        assert_eq!(parts.domain, "org");
        assert_eq!(parts.resource, "node");
        assert_eq!(parts.action, "create");
        assert!(parts.scope.is_none());
    }

    #[test]
    fn test_parse_capability_key_own_scope() {
        let cap = Capability::new("crm.visit:edit:own".to_string());
        let parts = cap.parse_key().unwrap();
        assert!(parts.is_own_scope());
        assert!(!parts.is_subtree_scope());
    }
}
