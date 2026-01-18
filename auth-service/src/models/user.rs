//! User model - tenant-scoped user accounts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// User state codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserState {
    Active,
    Suspended,
    Deactivated,
}

impl UserState {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserState::Active => "active",
            UserState::Suspended => "suspended",
            UserState::Deactivated => "deactivated",
        }
    }
}

/// User entity (tenant-scoped).
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub google_id: Option<String>,
    pub display_name: Option<String>,
    pub user_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl User {
    /// Create a new user.
    pub fn new(tenant_id: Uuid, email: String, display_name: Option<String>) -> Self {
        Self {
            user_id: Uuid::new_v4(),
            tenant_id,
            email,
            email_verified: false,
            google_id: None,
            display_name,
            user_state_code: UserState::Active.as_str().to_string(),
            created_utc: Utc::now(),
        }
    }

    /// Check if user is active.
    pub fn is_active(&self) -> bool {
        self.user_state_code == UserState::Active.as_str()
    }

    /// Convert to sanitized response (no sensitive fields).
    pub fn sanitized(&self) -> UserResponse {
        UserResponse::from(self.clone())
    }
}

/// Request to register a new user.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterUserRequest {
    pub tenant_id: Uuid,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// Request to login with email/password.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub tenant_id: Uuid,
    pub email: String,
    pub password: String,
}

/// User response for API (without sensitive fields).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub google_id: Option<String>,
    pub display_name: Option<String>,
    pub user_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            user_id: u.user_id,
            tenant_id: u.tenant_id,
            email: u.email,
            email_verified: u.email_verified,
            google_id: u.google_id,
            display_name: u.display_name,
            user_state_code: u.user_state_code,
            created_utc: u.created_utc,
        }
    }
}

/// Token pair response after successful auth.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

impl TokenResponse {
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_in,
            token_type: "Bearer".to_string(),
        }
    }
}

/// Auth response with user info and tokens.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tokens: TokenResponse,
}
