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
