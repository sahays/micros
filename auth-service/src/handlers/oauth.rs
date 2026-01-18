//! Google OAuth handlers for auth-service v2.
//!
//! Implements Google OAuth 2.0 login flow:
//! - OAuth URL generation
//! - Callback handling with code exchange
//! - ID token verification (direct token submission)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
    Json,
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

/// Query params for OAuth initiation.
#[derive(Debug, Deserialize)]
pub struct GoogleOAuthQuery {
    pub tenant_id: Uuid,
    pub redirect_uri: Option<String>,
}

/// Query params from Google callback.
#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Request to exchange Google ID token directly.
#[derive(Debug, Deserialize)]
pub struct GoogleTokenRequest {
    pub tenant_id: Uuid,
    pub id_token: String,
}

/// Response from Google token endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleTokenResponse {
    pub access_token: String,
    pub id_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
}

/// Google ID token claims.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleIdTokenClaims {
    pub sub: String, // Google user ID
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub aud: String, // Client ID
    pub iss: String, // Issuer
    pub exp: i64,    // Expiration
}

/// Response after successful Google auth.
#[derive(Debug, Serialize)]
pub struct GoogleAuthResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub is_new_user: bool,
}

/// OAuth state stored during flow.
#[derive(Debug, Serialize, Deserialize)]
struct OAuthState {
    pub tenant_id: Uuid,
    pub redirect_uri: Option<String>,
    pub nonce: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Initiate Google OAuth flow.
///
/// GET /auth/google
#[tracing::instrument(skip(state), fields(tenant_id = %query.tenant_id))]
pub async fn google_oauth_redirect(
    State(state): State<AppState>,
    Query(query): Query<GoogleOAuthQuery>,
) -> Result<Redirect, AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(query.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Generate state parameter (contains tenant_id and nonce for CSRF protection)
    let oauth_state = OAuthState {
        tenant_id: query.tenant_id,
        redirect_uri: query.redirect_uri,
        nonce: Uuid::new_v4().to_string(),
    };
    let state_json = serde_json::to_string(&oauth_state).map_err(|e| {
        AppError::InternalError(anyhow::anyhow!("Failed to serialize state: {}", e))
    })?;
    let state_encoded = base64_url_encode(&state_json);

    // Build Google OAuth URL
    let google_client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| AppError::ConfigError(anyhow::anyhow!("GOOGLE_CLIENT_ID not configured")))?;
    let google_redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").map_err(|_| {
        AppError::ConfigError(anyhow::anyhow!("GOOGLE_REDIRECT_URI not configured"))
    })?;

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&prompt=select_account",
        urlencoding::encode(&google_client_id),
        urlencoding::encode(&google_redirect_uri),
        urlencoding::encode(&state_encoded),
    );

    Ok(Redirect::to(&auth_url))
}

/// Handle Google OAuth callback.
///
/// GET /auth/google/callback
#[tracing::instrument(skip_all)]
pub async fn google_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<Redirect, AppError> {
    // Check for OAuth errors
    if let Some(error) = query.error {
        tracing::warn!(error = %error, "Google OAuth error");
        return Ok(Redirect::to(&format!(
            "/auth/error?error={}",
            urlencoding::encode(&error)
        )));
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Missing authorization code")))?;
    let state_encoded = query
        .state
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Missing state parameter")))?;

    // Decode and parse state
    let state_json = base64_url_decode(&state_encoded)
        .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid state parameter")))?;
    let oauth_state: OAuthState = serde_json::from_str(&state_json)
        .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid state format")))?;

    // Exchange code for tokens
    let tokens = exchange_code_for_tokens(&code).await?;

    // Get ID token
    let id_token = tokens
        .id_token
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("No ID token in response")))?;

    // Process the authentication
    let (user, is_new_user) = process_google_auth(&state, oauth_state.tenant_id, &id_token).await?;

    // Generate our tokens
    let (access_token, refresh_token, refresh_token_id) = state
        .jwt
        .generate_token_pair(
            &user.user_id.to_string(),
            &oauth_state.tenant_id.to_string(),
            "",
            &user.email,
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    // Store refresh session
    let refresh_hash = hash_string(&refresh_token_id);
    let session = RefreshSession::new(
        user.user_id,
        refresh_hash,
        state.jwt.refresh_token_expiry_days(),
    );
    state
        .db
        .insert_refresh_session(&session)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Redirect to frontend with tokens
    // In production, use a more secure method (e.g., short-lived auth code)
    let redirect_uri = oauth_state.redirect_uri.unwrap_or_else(|| "/".to_string());
    let redirect_url = format!(
        "{}?access_token={}&refresh_token={}&user_id={}&is_new_user={}",
        redirect_uri,
        urlencoding::encode(&access_token),
        urlencoding::encode(&refresh_token),
        user.user_id,
        is_new_user,
    );

    Ok(Redirect::to(&redirect_url))
}

/// Exchange Google ID token directly for auth tokens.
///
/// POST /auth/google/token
#[tracing::instrument(skip(state, req), fields(tenant_id = %req.tenant_id))]
pub async fn google_token_exchange(
    State(state): State<AppState>,
    Json(req): Json<GoogleTokenRequest>,
) -> Result<(StatusCode, Json<GoogleAuthResponse>), AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(req.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Process the authentication
    let (user, is_new_user) = process_google_auth(&state, req.tenant_id, &req.id_token).await?;

    // Generate our tokens
    let (access_token, refresh_token, refresh_token_id) = state
        .jwt
        .generate_token_pair(
            &user.user_id.to_string(),
            &req.tenant_id.to_string(),
            "",
            &user.email,
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    // Store refresh session
    let refresh_hash = hash_string(&refresh_token_id);
    let session = RefreshSession::new(
        user.user_id,
        refresh_hash,
        state.jwt.refresh_token_expiry_days(),
    );
    state
        .db
        .insert_refresh_session(&session)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(GoogleAuthResponse {
            user_id: user.user_id,
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt.access_token_expiry_seconds(),
            is_new_user,
        }),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Exchange authorization code for tokens.
async fn exchange_code_for_tokens(code: &str) -> Result<GoogleTokenResponse, AppError> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| AppError::ConfigError(anyhow::anyhow!("GOOGLE_CLIENT_ID not configured")))?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| {
        AppError::ConfigError(anyhow::anyhow!("GOOGLE_CLIENT_SECRET not configured"))
    })?;
    let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").map_err(|_| {
        AppError::ConfigError(anyhow::anyhow!("GOOGLE_REDIRECT_URI not configured"))
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| AppError::BadGateway(format!("Failed to contact Google: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!(error = %error_text, "Google token exchange failed");
        return Err(AppError::BadGateway(
            "Google token exchange failed".to_string(),
        ));
    }

    response
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|e| AppError::BadGateway(format!("Failed to parse Google response: {}", e)))
}

/// Process Google authentication - verify token and create/link user.
async fn process_google_auth(
    state: &AppState,
    tenant_id: Uuid,
    id_token: &str,
) -> Result<(User, bool), AppError> {
    // Decode and verify the ID token
    // Note: In production, verify signature against Google's public keys
    let claims = decode_google_id_token(id_token)?;

    // Verify audience matches our client ID
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| AppError::ConfigError(anyhow::anyhow!("GOOGLE_CLIENT_ID not configured")))?;
    if claims.aud != client_id {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Invalid token audience"
        )));
    }

    // Get email from claims
    let email = claims
        .email
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Email not provided by Google")))?;

    // Check if Google identity already exists
    if let Some(identity) = state
        .db
        .find_user_identity_by_subject(tenant_id, &IdentProvider::Google, &claims.sub)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
    {
        // User already has Google identity - find user
        let user = state
            .db
            .find_user_by_id(identity.user_id)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
            .ok_or_else(|| {
                AppError::InternalError(anyhow::anyhow!("User not found for identity"))
            })?;
        return Ok((user, false));
    }

    // Check if email already exists in tenant
    if let Some(user) = state
        .db
        .find_user_by_email_in_tenant(tenant_id, &email)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
    {
        // Link Google identity to existing user
        let identity = UserIdentity::new_google(user.user_id, claims.sub.clone());
        state
            .db
            .insert_user_identity(&identity)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

        // Mark email as verified (Google has verified it)
        state
            .db
            .update_user_email_verified(user.user_id, true)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

        return Ok((user, false));
    }

    // Create new user with Google identity
    let user = User::new(tenant_id, email.clone(), claims.name.clone());
    let user = User {
        email_verified: true, // Google has verified the email
        google_id: Some(claims.sub.clone()),
        ..user
    };

    state
        .db
        .insert_user(&user)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Create Google identity
    let identity = UserIdentity::new_google(user.user_id, claims.sub.clone());
    state
        .db
        .insert_user_identity(&identity)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((user, true))
}

/// Decode Google ID token (JWT).
/// Note: This does basic decoding without signature verification.
/// In production, verify against Google's public keys from https://www.googleapis.com/oauth2/v3/certs
fn decode_google_id_token(id_token: &str) -> Result<GoogleIdTokenClaims, AppError> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Invalid ID token format"
        )));
    }

    let payload = base64_url_decode(parts[1])
        .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Failed to decode token payload")))?;

    serde_json::from_str::<GoogleIdTokenClaims>(&payload)
        .map_err(|e| AppError::BadRequest(anyhow::anyhow!("Failed to parse token claims: {}", e)))
}

/// Base64 URL-safe encode.
fn base64_url_encode(input: &str) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(input.as_bytes())
}

/// Base64 URL-safe decode.
fn base64_url_decode(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let bytes = URL_SAFE_NO_PAD.decode(input)?;
    Ok(String::from_utf8(bytes)?)
}

/// Hash a string using SHA256.
fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
