//! Service registry handlers for auth-service v2.
//!
//! Implements Know-Your-Service (KYS) for service-to-service auth:
//! - Service registration with svc_key + svc_secret
//! - Service authentication for tokens
//! - Permission grants for service capabilities

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::handlers::auth::MessageResponse;
use crate::models::{Service, ServiceSecret};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to register a new service.
#[derive(Debug, Deserialize)]
pub struct RegisterServiceRequest {
    pub tenant_id: Option<Uuid>,
    pub svc_key: String,
    pub svc_label: String,
}

/// Service registration response with secret (shown only once).
#[derive(Debug, Serialize)]
pub struct ServiceRegistrationResponse {
    pub svc_id: Uuid,
    pub svc_key: String,
    pub svc_label: String,
    pub svc_secret: String, // Only shown once at registration
    pub secret_id: Uuid,
    pub created_utc: DateTime<Utc>,
}

/// Service info response (without secret).
#[derive(Debug, Serialize)]
pub struct ServiceResponse {
    pub svc_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub svc_key: String,
    pub svc_label: String,
    pub svc_state_code: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Service> for ServiceResponse {
    fn from(s: Service) -> Self {
        Self {
            svc_id: s.svc_id,
            tenant_id: s.tenant_id,
            svc_key: s.svc_key,
            svc_label: s.svc_label,
            svc_state_code: s.svc_state_code,
            created_utc: s.created_utc,
        }
    }
}

/// Request for service token.
#[derive(Debug, Deserialize)]
pub struct ServiceTokenRequest {
    pub svc_key: String,
    pub svc_secret: String,
}

/// Service token response.
#[derive(Debug, Serialize)]
pub struct ServiceTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub svc_id: Uuid,
    pub svc_key: String,
    pub permissions: Vec<String>,
}

/// Request to rotate secret.
#[derive(Debug, Deserialize)]
pub struct RotateSecretRequest {
    pub current_secret: String,
}

/// Rotated secret response.
#[derive(Debug, Serialize)]
pub struct RotatedSecretResponse {
    pub svc_id: Uuid,
    pub new_secret_id: Uuid,
    pub new_secret: String,
    pub old_secret_valid_until: DateTime<Utc>,
}

/// Request to grant permission to service.
#[derive(Debug, Deserialize)]
pub struct GrantPermissionRequest {
    pub permission: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new service.
/// This is an admin endpoint - should be protected.
///
/// POST /services
pub async fn register_service(
    State(state): State<AppState>,
    Json(req): Json<RegisterServiceRequest>,
) -> Result<(StatusCode, Json<ServiceRegistrationResponse>), AppError> {
    // Check if service key already exists
    if state
        .db
        .find_service_by_key(&req.svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .is_some()
    {
        return Err(AppError::Conflict(anyhow::anyhow!(
            "Service key already exists"
        )));
    }

    // Generate service secret
    let secret = generate_secret();
    let secret_hash = hash_secret(&secret);

    // Create service
    let service = Service::new(req.tenant_id, req.svc_key.clone(), req.svc_label.clone());

    state
        .db
        .insert_service(&service)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Create secret
    let service_secret = ServiceSecret::new(service.svc_id, secret_hash);
    let secret_id = service_secret.secret_id;

    state
        .db
        .insert_service_secret(&service_secret)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(ServiceRegistrationResponse {
            svc_id: service.svc_id,
            svc_key: service.svc_key,
            svc_label: service.svc_label,
            svc_secret: secret, // Only shown once
            secret_id,
            created_utc: service.created_utc,
        }),
    ))
}

/// Get service by key.
///
/// GET /services/:svc_key
pub async fn get_service(
    State(state): State<AppState>,
    Path(svc_key): Path<String>,
) -> Result<Json<ServiceResponse>, AppError> {
    let service = state
        .db
        .find_service_by_key(&svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Service not found")))?;

    Ok(Json(ServiceResponse::from(service)))
}

/// Get service token using svc_key + svc_secret.
///
/// POST /services/token
pub async fn get_service_token(
    State(state): State<AppState>,
    Json(req): Json<ServiceTokenRequest>,
) -> Result<Json<ServiceTokenResponse>, AppError> {
    // Find service
    let service = state
        .db
        .find_service_by_key(&req.svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    if !service.is_active() {
        return Err(AppError::AuthError(anyhow::anyhow!("Service is disabled")));
    }

    // Get valid secret and verify
    let secret_hash = hash_secret(&req.svc_secret);
    let valid_secret = state
        .db
        .find_valid_service_secret(service.svc_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid credentials")))?;

    // Compare secret hashes
    if valid_secret.secret_hash_text != secret_hash {
        return Err(AppError::AuthError(anyhow::anyhow!("Invalid credentials")));
    }

    // Get service permissions
    let permissions = state
        .db
        .get_service_permissions(service.svc_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Generate app token
    let token = state
        .jwt
        .generate_app_token(
            &service.svc_id.to_string(),
            &service.svc_label,
            permissions.clone(),
            0, // No rate limit for services
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    let expires_in = state.config.jwt.app_token_expiry_minutes * 60;

    Ok(Json(ServiceTokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        svc_id: service.svc_id,
        svc_key: service.svc_key,
        permissions,
    }))
}

/// Rotate service secret.
///
/// POST /services/:svc_key/rotate-secret
pub async fn rotate_secret(
    State(state): State<AppState>,
    Path(svc_key): Path<String>,
    Json(req): Json<RotateSecretRequest>,
) -> Result<Json<RotatedSecretResponse>, AppError> {
    // Find service
    let service = state
        .db
        .find_service_by_key(&svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Service not found")))?;

    // Verify current secret
    let current_hash = hash_secret(&req.current_secret);
    let current_secret = state
        .db
        .find_valid_service_secret(service.svc_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("No valid secret found")))?;

    if current_secret.secret_hash_text != current_hash {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Invalid current secret"
        )));
    }

    // Generate new secret
    let new_secret = generate_secret();
    let new_hash = hash_secret(&new_secret);

    // Create new secret record
    let new_service_secret = ServiceSecret::new(service.svc_id, new_hash);
    let new_secret_id = new_service_secret.secret_id;

    state
        .db
        .insert_service_secret(&new_service_secret)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Schedule old secret for revocation (give 24h grace period)
    let old_valid_until = Utc::now() + Duration::hours(24);

    Ok(Json(RotatedSecretResponse {
        svc_id: service.svc_id,
        new_secret_id,
        new_secret,
        old_secret_valid_until: old_valid_until,
    }))
}

/// Grant permission to a service.
///
/// POST /services/:svc_key/permissions
pub async fn grant_permission(
    State(state): State<AppState>,
    Path(svc_key): Path<String>,
    Json(req): Json<GrantPermissionRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    // Find service
    let service = state
        .db
        .find_service_by_key(&svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Service not found")))?;

    // Grant permission
    state
        .db
        .insert_service_permission(service.svc_id, &req.permission)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(MessageResponse {
        message: format!("Permission '{}' granted to service", req.permission),
    }))
}

/// Get permissions for a service.
///
/// GET /services/:svc_key/permissions
pub async fn get_service_permissions(
    State(state): State<AppState>,
    Path(svc_key): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    // Find service
    let service = state
        .db
        .find_service_by_key(&svc_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Service not found")))?;

    let permissions = state
        .db
        .get_service_permissions(service.svc_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(permissions))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a secure random secret.
fn generate_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const SECRET_LEN: usize = 48;

    let mut rng = rand::thread_rng();
    (0..SECRET_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash a secret for storage.
fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}
