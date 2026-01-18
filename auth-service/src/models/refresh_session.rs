//! Refresh session model - token sessions for JWT refresh.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Refresh session entity.
#[derive(Debug, Clone, FromRow)]
pub struct RefreshSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub token_hash_text: String,
    pub expiry_utc: DateTime<Utc>,
    pub revoked_utc: Option<DateTime<Utc>>,
    pub created_utc: DateTime<Utc>,
}

impl RefreshSession {
    /// Create a new refresh session.
    pub fn new(user_id: Uuid, token_hash: String, expiry_days: i64) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            user_id,
            token_hash_text: token_hash,
            expiry_utc: Utc::now() + Duration::days(expiry_days),
            revoked_utc: None,
            created_utc: Utc::now(),
        }
    }

    /// Check if session is valid (not expired, not revoked).
    pub fn is_valid(&self) -> bool {
        self.revoked_utc.is_none() && self.expiry_utc > Utc::now()
    }

    /// Check if session is expired.
    pub fn is_expired(&self) -> bool {
        self.expiry_utc <= Utc::now()
    }

    /// Check if session is revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked_utc.is_some()
    }
}

/// Session info for API responses.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub created_utc: DateTime<Utc>,
    pub expiry_utc: DateTime<Utc>,
    pub is_current: bool,
}

impl From<RefreshSession> for SessionInfo {
    fn from(s: RefreshSession) -> Self {
        Self {
            session_id: s.session_id,
            created_utc: s.created_utc,
            expiry_utc: s.expiry_utc,
            is_current: false, // Set by caller
        }
    }
}
