use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    EmailVerification,
    PasswordReset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationToken {
    #[serde(rename = "_id")]
    pub id: String,
    pub token: String,
    pub user_id: String,
    pub token_type: TokenType,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl VerificationToken {
    pub fn new_email_verification(user_id: String, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            token,
            user_id,
            token_type: TokenType::EmailVerification,
            expires_at: now + Duration::hours(24), // 24 hours expiry
            created_at: now,
        }
    }

    pub fn new_password_reset(user_id: String, token: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            token,
            user_id,
            token_type: TokenType::PasswordReset,
            expires_at: now + Duration::hours(1), // 1 hour expiry
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_verification_token_creation() {
        let user_id = "user_123".to_string();
        let token_str = "token_abc".to_string();
        let token = VerificationToken::new_email_verification(user_id.clone(), token_str.clone());

        assert_eq!(token.user_id, user_id);
        assert_eq!(token.token, token_str);
        assert!(matches!(token.token_type, TokenType::EmailVerification));
        assert!(!token.is_expired());
    }

    #[test]
    fn test_password_reset_token_creation() {
        let user_id = "user_123".to_string();
        let token_str = "token_abc".to_string();
        let token = VerificationToken::new_password_reset(user_id.clone(), token_str.clone());

        assert_eq!(token.user_id, user_id);
        assert_eq!(token.token, token_str);
        assert!(matches!(token.token_type, TokenType::PasswordReset));
        assert!(!token.is_expired());
    }

    #[test]
    fn test_token_expiration() {
        let mut token = VerificationToken::new_password_reset("u".to_string(), "t".to_string());
        
        // Manually set expiry to past
        token.expires_at = Utc::now() - Duration::hours(1);
        assert!(token.is_expired());

        // Manually set expiry to future
        token.expires_at = Utc::now() + Duration::hours(1);
        assert!(!token.is_expired());
    }
}
