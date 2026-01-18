//! User identity model - authentication providers (password, google, etc.)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Identity provider codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentProvider {
    Password,
    Google,
}

impl IdentProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            IdentProvider::Password => "password",
            IdentProvider::Google => "google",
        }
    }
}

impl std::str::FromStr for IdentProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "password" => Ok(IdentProvider::Password),
            "google" => Ok(IdentProvider::Google),
            _ => Err(format!("Invalid identity provider: {}", s)),
        }
    }
}

/// User identity entity.
/// For password auth, ident_hash stores the password hash.
/// For Google auth, ident_hash stores the Google subject ID.
#[derive(Debug, Clone, FromRow)]
pub struct UserIdentity {
    pub ident_id: Uuid,
    pub user_id: Uuid,
    pub ident_provider_code: String,
    pub ident_hash: String,
    pub created_utc: DateTime<Utc>,
}

impl UserIdentity {
    /// Create a new password identity.
    pub fn new_password(user_id: Uuid, password_hash: String) -> Self {
        Self {
            ident_id: Uuid::new_v4(),
            user_id,
            ident_provider_code: IdentProvider::Password.as_str().to_string(),
            ident_hash: password_hash,
            created_utc: Utc::now(),
        }
    }

    /// Create a new Google identity.
    pub fn new_google(user_id: Uuid, google_sub: String) -> Self {
        Self {
            ident_id: Uuid::new_v4(),
            user_id,
            ident_provider_code: IdentProvider::Google.as_str().to_string(),
            ident_hash: google_sub,
            created_utc: Utc::now(),
        }
    }

    /// Check if this is a password identity.
    pub fn is_password(&self) -> bool {
        self.ident_provider_code == IdentProvider::Password.as_str()
    }

    /// Check if this is a Google identity.
    pub fn is_google(&self) -> bool {
        self.ident_provider_code == IdentProvider::Google.as_str()
    }
}
