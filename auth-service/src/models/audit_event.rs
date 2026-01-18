//! Audit event model - security and compliance logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Audit event types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    UserRegistered,
    UserLogin,
    UserLogout,
    UserPasswordChanged,
    UserEmailVerified,
    UserSuspended,
    UserDeactivated,
    UserReactivated,
    TokenRefreshed,
    TokenRevoked,
    OrgNodeCreated,
    OrgNodeUpdated,
    OrgNodeDeactivated,
    RoleCreated,
    RoleUpdated,
    RoleDeleted,
    CapabilityAssigned,
    CapabilityRevoked,
    AssignmentCreated,
    AssignmentEnded,
    ServiceRegistered,
    ServiceSecretRotated,
    ServiceDisabled,
    AuthzEvaluated,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::UserRegistered => "user_registered",
            AuditEventType::UserLogin => "user_login",
            AuditEventType::UserLogout => "user_logout",
            AuditEventType::UserPasswordChanged => "user_password_changed",
            AuditEventType::UserEmailVerified => "user_email_verified",
            AuditEventType::UserSuspended => "user_suspended",
            AuditEventType::UserDeactivated => "user_deactivated",
            AuditEventType::UserReactivated => "user_reactivated",
            AuditEventType::TokenRefreshed => "token_refreshed",
            AuditEventType::TokenRevoked => "token_revoked",
            AuditEventType::OrgNodeCreated => "org_node_created",
            AuditEventType::OrgNodeUpdated => "org_node_updated",
            AuditEventType::OrgNodeDeactivated => "org_node_deactivated",
            AuditEventType::RoleCreated => "role_created",
            AuditEventType::RoleUpdated => "role_updated",
            AuditEventType::RoleDeleted => "role_deleted",
            AuditEventType::CapabilityAssigned => "capability_assigned",
            AuditEventType::CapabilityRevoked => "capability_revoked",
            AuditEventType::AssignmentCreated => "assignment_created",
            AuditEventType::AssignmentEnded => "assignment_ended",
            AuditEventType::ServiceRegistered => "service_registered",
            AuditEventType::ServiceSecretRotated => "service_secret_rotated",
            AuditEventType::ServiceDisabled => "service_disabled",
            AuditEventType::AuthzEvaluated => "authz_evaluated",
        }
    }
}

/// Audit event entity.
#[derive(Debug, Clone, FromRow)]
pub struct AuditEvent {
    pub event_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    pub actor_svc_id: Option<Uuid>,
    pub event_type_code: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub event_data: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_utc: DateTime<Utc>,
}

impl AuditEvent {
    /// Create a new audit event for a user action.
    #[allow(clippy::too_many_arguments)]
    pub fn user_action(
        tenant_id: Uuid,
        actor_user_id: Uuid,
        event_type: AuditEventType,
        target_type: Option<String>,
        target_id: Option<Uuid>,
        event_data: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            tenant_id: Some(tenant_id),
            actor_user_id: Some(actor_user_id),
            actor_svc_id: None,
            event_type_code: event_type.as_str().to_string(),
            target_type,
            target_id,
            event_data,
            ip_address,
            user_agent,
            created_utc: Utc::now(),
        }
    }

    /// Create a new audit event for a service action.
    pub fn service_action(
        tenant_id: Option<Uuid>,
        actor_svc_id: Uuid,
        event_type: AuditEventType,
        target_type: Option<String>,
        target_id: Option<Uuid>,
        event_data: Option<serde_json::Value>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            tenant_id,
            actor_user_id: None,
            actor_svc_id: Some(actor_svc_id),
            event_type_code: event_type.as_str().to_string(),
            target_type,
            target_id,
            event_data,
            ip_address,
            user_agent: None,
            created_utc: Utc::now(),
        }
    }

    /// Create a system-level audit event (no actor).
    pub fn system_action(
        event_type: AuditEventType,
        target_type: Option<String>,
        target_id: Option<Uuid>,
        event_data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            tenant_id: None,
            actor_user_id: None,
            actor_svc_id: None,
            event_type_code: event_type.as_str().to_string(),
            target_type,
            target_id,
            event_data,
            ip_address: None,
            user_agent: None,
            created_utc: Utc::now(),
        }
    }
}

/// Audit event response for API.
#[derive(Debug, Serialize)]
pub struct AuditEventResponse {
    pub event_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    pub actor_svc_id: Option<Uuid>,
    pub event_type_code: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub event_data: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
}

impl From<AuditEvent> for AuditEventResponse {
    fn from(e: AuditEvent) -> Self {
        Self {
            event_id: e.event_id,
            tenant_id: e.tenant_id,
            actor_user_id: e.actor_user_id,
            actor_svc_id: e.actor_svc_id,
            event_type_code: e.event_type_code,
            target_type: e.target_type,
            target_id: e.target_id,
            event_data: e.event_data,
            created_utc: e.created_utc,
        }
    }
}
