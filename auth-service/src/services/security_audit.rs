//! Security audit service for tracking tenant isolation and access attempts.
//!
//! Logs security-relevant events such as:
//! - Cross-tenant access attempts
//! - Authentication failures
//! - Suspicious activity patterns

use chrono::{DateTime, Utc};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::MongoDb;

/// Security audit event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    /// Attempted access to resource in different tenant
    CrossTenantAccess,
    /// Attempted access to disabled organization
    DisabledOrgAccess,
    /// Multiple failed login attempts
    BruteForceAttempt,
    /// Invalid or expired token used
    InvalidTokenUsage,
    /// Unauthorized admin action attempt
    UnauthorizedAdminAction,
    /// Suspicious request pattern detected
    SuspiciousActivity,
}

/// Security audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditLog {
    /// Unique ID for this audit entry
    #[serde(rename = "_id")]
    pub id: String,
    /// Type of security event
    pub event_type: SecurityEventType,
    /// Severity level: info, warning, critical
    pub severity: String,
    /// App ID where the event occurred (if known)
    pub app_id: Option<String>,
    /// Org ID where the event occurred (if known)
    pub org_id: Option<String>,
    /// User ID involved (if known)
    pub user_id: Option<String>,
    /// IP address of the requester
    pub ip_address: String,
    /// Request path
    pub request_path: String,
    /// HTTP method
    pub request_method: String,
    /// Additional context about the event
    pub details: String,
    /// Timestamp of the event
    pub created_at: DateTime<Utc>,
}

impl SecurityAuditLog {
    /// Create a new security audit log entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_type: SecurityEventType,
        severity: impl Into<String>,
        app_id: Option<String>,
        org_id: Option<String>,
        user_id: Option<String>,
        ip_address: impl Into<String>,
        request_path: impl Into<String>,
        request_method: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            severity: severity.into(),
            app_id,
            org_id,
            user_id,
            ip_address: ip_address.into(),
            request_path: request_path.into(),
            request_method: request_method.into(),
            details: details.into(),
            created_at: Utc::now(),
        }
    }

    /// Create a cross-tenant access attempt log.
    #[allow(clippy::too_many_arguments)]
    pub fn cross_tenant_access(
        claimed_app_id: &str,
        claimed_org_id: &str,
        target_app_id: &str,
        target_org_id: &str,
        user_id: Option<&str>,
        ip_address: &str,
        request_path: &str,
        request_method: &str,
    ) -> Self {
        Self::new(
            SecurityEventType::CrossTenantAccess,
            "critical",
            Some(claimed_app_id.to_string()),
            Some(claimed_org_id.to_string()),
            user_id.map(|s| s.to_string()),
            ip_address,
            request_path,
            request_method,
            format!(
                "Attempted access to tenant ({}, {}) from tenant ({}, {})",
                target_app_id, target_org_id, claimed_app_id, claimed_org_id
            ),
        )
    }

    /// Create a disabled org access attempt log.
    pub fn disabled_org_access(
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        ip_address: &str,
        request_path: &str,
        request_method: &str,
    ) -> Self {
        Self::new(
            SecurityEventType::DisabledOrgAccess,
            "warning",
            Some(app_id.to_string()),
            Some(org_id.to_string()),
            user_id.map(|s| s.to_string()),
            ip_address,
            request_path,
            request_method,
            format!("Attempted access to disabled organization: {}", org_id),
        )
    }
}

/// Security audit service for logging security events.
#[derive(Clone)]
pub struct SecurityAuditService {
    db: MongoDb,
}

impl SecurityAuditService {
    /// Create a new security audit service.
    pub fn new(db: MongoDb) -> Self {
        Self { db }
    }

    /// Log a security event asynchronously (non-blocking).
    pub fn log_async(&self, log: SecurityAuditLog) {
        let db = self.db.clone();
        tokio::spawn(async move {
            if let Err(e) = db.security_audit_logs().insert_one(&log, None).await {
                tracing::error!(
                    error = %e,
                    event_type = ?log.event_type,
                    "Failed to write security audit log"
                );
            } else {
                tracing::warn!(
                    event_type = ?log.event_type,
                    severity = %log.severity,
                    details = %log.details,
                    "Security event logged"
                );
            }
        });
    }

    /// Log a security event synchronously.
    pub async fn log(&self, log: SecurityAuditLog) -> Result<(), mongodb::error::Error> {
        tracing::warn!(
            event_type = ?log.event_type,
            severity = %log.severity,
            details = %log.details,
            "Security event"
        );
        self.db.security_audit_logs().insert_one(&log, None).await?;
        Ok(())
    }
}
