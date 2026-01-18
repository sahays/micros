//! Authentication handlers for auth-service v2.
//!
//! Implements password-based authentication with:
//! - Registration with email verification
//! - Login with JWT tokens
//! - Token refresh
//! - Logout with token blacklisting

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{IdentProvider, RefreshSession, User, UserIdentity};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Registration request.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// Login request.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
}

/// Token refresh request.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Logout request.
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// Authentication response with tokens.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

/// User information in auth response.
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub tenant_id: Uuid,
}

/// Message response for simple operations.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new user.
///
/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // Validate input
    validate_email(&req.email)?;
    validate_password(&req.password)?;

    // Find tenant by slug
    let tenant = state
        .db
        .find_tenant_by_slug(&req.tenant_slug)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Check if tenant is active
    if !tenant.is_active() {
        return Err(AppError::BadRequest(anyhow::anyhow!("Tenant is suspended")));
    }

    // Check if email already registered
    if state
        .db
        .find_user_by_email_in_tenant(tenant.tenant_id, &req.email)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .is_some()
    {
        return Err(AppError::Conflict(anyhow::anyhow!(
            "Email already registered"
        )));
    }

    // Hash password
    let password_hash = hash_password(&req.password)?;

    // Create user
    let display_name = req
        .display_name
        .unwrap_or_else(|| req.email.split('@').next().unwrap_or("User").to_string());
    let user = User::new(tenant.tenant_id, req.email.clone(), Some(display_name));

    state
        .db
        .insert_user(&user)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Create password identity
    let identity = UserIdentity::new_password(user.user_id, password_hash);
    state
        .db
        .insert_user_identity(&identity)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Generate tokens
    let (access_token, refresh_token, expires_in) =
        generate_tokens(&state, &user, tenant.tenant_id).await?;

    let response = AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserResponse {
            user_id: user.user_id,
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            tenant_id: user.tenant_id,
        },
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Login with email and password.
///
/// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Find tenant
    let tenant = state
        .db
        .find_tenant_by_slug(&req.tenant_slug)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    if !tenant.is_active() {
        return Err(AppError::BadRequest(anyhow::anyhow!("Tenant is suspended")));
    }

    // Find user
    let user = state
        .db
        .find_user_by_email_in_tenant(tenant.tenant_id, &req.email)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    // Check if user is active
    if !user.is_active() {
        return Err(AppError::AuthError(anyhow::anyhow!("Account is disabled")));
    }

    // Find password identity
    let identity = state
        .db
        .find_user_identity(user.user_id, IdentProvider::Password.as_str())
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    // Verify password
    if !verify_password(&req.password, &identity.ident_hash)? {
        // TODO: Add failed login tracking when needed
        return Err(AppError::AuthError(anyhow::anyhow!("Invalid credentials")));
    }

    // Generate tokens
    let (access_token, refresh_token, expires_in) =
        generate_tokens(&state, &user, tenant.tenant_id).await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserResponse {
            user_id: user.user_id,
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            tenant_id: user.tenant_id,
        },
    }))
}

/// Refresh access token using refresh token.
///
/// POST /auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Verify and decode refresh token
    let claims = state
        .jwt
        .validate_refresh_token(&req.refresh_token)
        .map_err(|e| AppError::AuthError(anyhow::anyhow!("Invalid refresh token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid token subject")))?;

    // Hash the token to look up session
    let token_hash = hash_token(&req.refresh_token);

    // Find refresh session
    let session = state
        .db
        .find_refresh_session_by_hash(&token_hash)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Session not found")))?;

    // Verify session belongs to the user from token
    if session.user_id != user_id {
        return Err(AppError::AuthError(anyhow::anyhow!("Session mismatch")));
    }

    // Check if session is valid
    if !session.is_valid() {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Session expired or revoked"
        )));
    }

    // Get user
    let user = state
        .db
        .find_user_by_id(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("User not found")))?;

    if !user.is_active() {
        return Err(AppError::AuthError(anyhow::anyhow!("Account is disabled")));
    }

    // Revoke old session
    state
        .db
        .revoke_refresh_session(session.session_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Generate new tokens
    let (access_token, refresh_token, expires_in) =
        generate_tokens(&state, &user, user.tenant_id).await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserResponse {
            user_id: user.user_id,
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            tenant_id: user.tenant_id,
        },
    }))
}

/// Logout and revoke refresh token.
///
/// POST /auth/logout
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    // Verify refresh token (but don't fail if invalid - still try to revoke)
    if let Ok(claims) = state.jwt.validate_refresh_token(&req.refresh_token) {
        let token_hash = hash_token(&req.refresh_token);

        // Try to find and revoke the session
        if let Ok(Some(session)) = state.db.find_refresh_session_by_hash(&token_hash).await {
            let _ = state.db.revoke_refresh_session(session.session_id).await;
        }

        // Also blacklist the access token using jti from refresh token
        let ttl = state.config.jwt.access_token_expiry_minutes * 60;
        let _ = state.redis.blacklist_token(&claims.jti, ttl).await;
    }

    Ok(Json(MessageResponse {
        message: "Logged out successfully".to_string(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate access and refresh tokens.
async fn generate_tokens(
    state: &AppState,
    user: &User,
    tenant_id: Uuid,
) -> Result<(String, String, i64), AppError> {
    // Generate token IDs
    let refresh_token_id = Uuid::new_v4().to_string();

    // Generate access token - using tenant_id as both app_id and org_id for now
    // These will be set properly once org hierarchy is implemented
    let access_token = state
        .jwt
        .generate_access_token(
            &user.user_id.to_string(),
            &tenant_id.to_string(),
            &tenant_id.to_string(), // org_id - will be set properly later
            &user.email,
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    // Generate refresh token
    let refresh_token = state
        .jwt
        .generate_refresh_token(&user.user_id.to_string(), &refresh_token_id)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    // Store refresh session
    let token_hash = hash_token(&refresh_token);
    let session = RefreshSession::new(
        user.user_id,
        token_hash,
        state.config.jwt.refresh_token_expiry_days,
    );

    state
        .db
        .insert_refresh_session(&session)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let expires_in = state.config.jwt.access_token_expiry_minutes * 60;

    Ok((access_token, refresh_token, expires_in))
}

/// Hash a token for storage (not for security, just for lookup).
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hash password using argon2.
fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Password hashing failed: {}", e)))
}

/// Verify password against stored hash.
fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Invalid password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Validate email format.
fn validate_email(email: &str) -> Result<(), AppError> {
    if email.is_empty() {
        return Err(AppError::BadRequest(anyhow::anyhow!("Email is required")));
    }
    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Invalid email format"
        )));
    }
    if email.len() > 255 {
        return Err(AppError::BadRequest(anyhow::anyhow!("Email too long")));
    }
    Ok(())
}

/// Validate password strength.
fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Password must be at least 8 characters"
        )));
    }
    if password.len() > 128 {
        return Err(AppError::BadRequest(anyhow::anyhow!("Password too long")));
    }
    Ok(())
}
