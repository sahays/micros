//! Invitation model - user invitations with pre-assigned roles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Invitation state codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationState {
    Pending,
    Accepted,
    Expired,
    Revoked,
}

impl InvitationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationState::Pending => "pending",
            InvitationState::Accepted => "accepted",
            InvitationState::Expired => "expired",
            InvitationState::Revoked => "revoked",
        }
    }
}

/// Invitation entity.
#[derive(Debug, Clone, FromRow)]
pub struct Invitation {
    pub invitation_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub token_hash: String,
    pub state_code: String,
    pub expiry_utc: DateTime<Utc>,
    pub accepted_utc: Option<DateTime<Utc>>,
    pub created_by_user_id: Uuid,
    pub created_utc: DateTime<Utc>,
}

impl Invitation {
    /// Create a new invitation.
    pub fn new(
        tenant_id: Uuid,
        email: String,
        org_node_id: Uuid,
        role_id: Uuid,
        token_hash: String,
        expiry_utc: DateTime<Utc>,
        created_by_user_id: Uuid,
    ) -> Self {
        Self {
            invitation_id: Uuid::new_v4(),
            tenant_id,
            email,
            org_node_id,
            role_id,
            token_hash,
            state_code: InvitationState::Pending.as_str().to_string(),
            expiry_utc,
            accepted_utc: None,
            created_by_user_id,
            created_utc: Utc::now(),
        }
    }

    /// Check if invitation is pending and not expired.
    pub fn is_valid(&self) -> bool {
        self.state_code == InvitationState::Pending.as_str() && Utc::now() < self.expiry_utc
    }

    /// Check if invitation has been accepted.
    pub fn is_accepted(&self) -> bool {
        self.state_code == InvitationState::Accepted.as_str()
    }

    /// Check if invitation has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expiry_utc
    }
}

/// Request to create an invitation.
#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub tenant_id: Uuid,
    pub email: String,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub expires_in_hours: Option<i64>,
}

/// Invitation response for API.
#[derive(Debug, Serialize)]
pub struct InvitationResponse {
    pub invitation_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub state_code: String,
    pub expiry_utc: DateTime<Utc>,
    pub created_utc: DateTime<Utc>,
}

impl From<Invitation> for InvitationResponse {
    fn from(i: Invitation) -> Self {
        Self {
            invitation_id: i.invitation_id,
            tenant_id: i.tenant_id,
            email: i.email,
            org_node_id: i.org_node_id,
            role_id: i.role_id,
            state_code: i.state_code,
            expiry_utc: i.expiry_utc,
            created_utc: i.created_utc,
        }
    }
}

/// Request to accept an invitation.
#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
    pub password: String,
    pub display_name: Option<String>,
}
