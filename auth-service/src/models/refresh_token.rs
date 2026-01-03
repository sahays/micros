use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Refresh token stored in MongoDB for session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    /// Unique identifier for the refresh token (jti claim)
    #[serde(rename = "_id")]
    pub id: String,

    /// User ID this token belongs to
    pub user_id: String,

    /// SHA-256 hash of the refresh token
    pub token_hash: String,

    /// When this token expires
    pub expires_at: DateTime<Utc>,

    /// When this token was created
    pub created_at: DateTime<Utc>,

    /// Whether this token has been revoked (for logout)
    #[serde(default)]
    pub revoked: bool,
}

impl RefreshToken {
    /// Create a new refresh token
    pub fn new(user_id: String, token: &str, expires_in_days: i64) -> Self {
        Self::new_with_id(Uuid::new_v4().to_string(), user_id, token, expires_in_days)
    }

    /// Create a new refresh token with a specific ID (useful when ID is needed for JWT claims)
    pub fn new_with_id(id: String, user_id: String, token: &str, expires_in_days: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::days(expires_in_days);
        let token_hash = Self::hash_token(token);

        Self {
            id,
            user_id,
            token_hash,
            expires_at,
            created_at: now,
            revoked: false,
        }
    }

    /// Hash a token using SHA-256
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if this token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if this token is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.revoked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_token_creation() {
        let token = RefreshToken::new("user_123".to_string(), "token_abc", 7);

        assert_eq!(token.user_id, "user_123");
        // Hash of "token_abc"
        assert_ne!(token.token_hash, "token_abc");
        assert!(!token.revoked);
        assert!(token.is_valid());
    }

    #[test]
    fn test_refresh_token_expiry() {
        let mut token = RefreshToken::new("user_123".to_string(), "token_abc", 7);

        // Not expired initially
        assert!(!token.is_expired());
        assert!(token.is_valid());

        // Simulate expiry
        token.expires_at = Utc::now() - Duration::seconds(1);
        assert!(token.is_expired());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_refresh_token_revocation() {
        let mut token = RefreshToken::new("user_123".to_string(), "token_abc", 7);

        assert!(token.is_valid());

        // Revoke token
        token.revoked = true;
        assert!(!token.is_valid());
    }
}
