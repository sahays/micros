//! Invitation handlers for auth-service v2.
//!
//! Implements user invitation flow:
//! - Create invitation with pre-assigned role/org
//! - Get invitation details by token
//! - Accept invitation (creates user and assignment)

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{
    CreateInvitationRequest, Invitation, OrgAssignment, RefreshSession, User, UserIdentity,
};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Response after creating an invitation.
#[derive(Debug, Serialize)]
pub struct CreateInvitationResponse {
    pub invitation_id: Uuid,
    pub invite_token: String,
    pub invite_url: String,
}

/// Invitation details for display.
#[derive(Debug, Serialize)]
pub struct InvitationDetailsResponse {
    pub invitation_id: Uuid,
    pub email: String,
    pub org_node_label: Option<String>,
    pub role_label: Option<String>,
    pub invited_by: Option<String>,
    pub expiry_utc: String,
    pub is_valid: bool,
}

/// Response after accepting an invitation.
#[derive(Debug, Serialize)]
pub struct AcceptInvitationResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Accept invitation via URL token.
#[derive(Debug, Deserialize)]
pub struct AcceptByTokenRequest {
    pub password: String,
    pub display_name: Option<String>,
}

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_EXPIRY_HOURS: i64 = 168; // 7 days

// ============================================================================
// Handlers
// ============================================================================

/// Create a new invitation.
///
/// POST /invitations
#[tracing::instrument(
    skip(state),
    fields(tenant_id = %req.tenant_id, org_node_id = %req.org_node_id, role_id = %req.role_id)
)]
pub async fn create_invitation(
    State(state): State<AppState>,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<CreateInvitationResponse>), AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(req.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Validate org node exists and belongs to tenant
    let org_node = state
        .db
        .find_org_node_by_id(req.org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Org node not found")))?;

    if org_node.tenant_id != req.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Org node does not belong to specified tenant"
        )));
    }

    // Validate role exists and belongs to tenant
    let role = state
        .db
        .find_role_by_id(req.role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Role not found")))?;

    if role.tenant_id != req.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Role does not belong to specified tenant"
        )));
    }

    // Check if user with this email already exists in tenant
    if state
        .db
        .find_user_by_email_in_tenant(req.tenant_id, &req.email)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .is_some()
    {
        return Err(AppError::Conflict(anyhow::anyhow!(
            "User with this email already exists in tenant"
        )));
    }

    // Generate secure token
    let token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&token);

    // Calculate expiry
    let expiry_hours = req.expires_in_hours.unwrap_or(DEFAULT_EXPIRY_HOURS);
    let expiry_utc = Utc::now() + Duration::hours(expiry_hours);

    // Create invitation (using system user ID for created_by_user_id since we don't have auth context)
    // In a real implementation, this would come from the authenticated user
    let system_user_id = Uuid::nil(); // Placeholder for system-created invitations

    let invitation = Invitation::new(
        req.tenant_id,
        req.email.clone(),
        req.org_node_id,
        req.role_id,
        token_hash,
        expiry_utc,
        system_user_id,
    );
    let invitation_id = invitation.invitation_id;

    state
        .db
        .insert_invitation(&invitation)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Build invite URL (in production, this would use a configured base URL)
    let invite_url = format!(
        "{}/auth/invitations/{}",
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:9005".to_string()),
        token
    );

    // Send invitation email (TODO: implement actual email sending)
    tracing::info!(
        email = %req.email,
        invitation_id = %invitation_id,
        "Invitation created"
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateInvitationResponse {
            invitation_id,
            invite_token: token,
            invite_url,
        }),
    ))
}

/// Get invitation details by token.
///
/// GET /invitations/{token}
#[tracing::instrument(skip_all)]
pub async fn get_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<InvitationDetailsResponse>, AppError> {
    let token_hash = hash_token(&token);

    // Find invitation
    let invitation = state
        .db
        .find_invitation_by_token_hash(&token_hash)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| {
            AppError::NotFound(anyhow::anyhow!("Invitation not found or already used"))
        })?;

    // Check if still valid
    let is_valid = invitation.is_valid();

    // Get org node label
    let org_node_label = state
        .db
        .find_org_node_by_id(invitation.org_node_id)
        .await
        .ok()
        .flatten()
        .map(|n| n.node_label);

    // Get role label
    let role_label = state
        .db
        .find_role_by_id(invitation.role_id)
        .await
        .ok()
        .flatten()
        .map(|r| r.role_label);

    // Get inviter name
    let invited_by = if invitation.created_by_user_id != Uuid::nil() {
        state
            .db
            .find_user_by_id(invitation.created_by_user_id)
            .await
            .ok()
            .flatten()
            .and_then(|u| u.display_name)
    } else {
        None
    };

    Ok(Json(InvitationDetailsResponse {
        invitation_id: invitation.invitation_id,
        email: invitation.email,
        org_node_label,
        role_label,
        invited_by,
        expiry_utc: invitation.expiry_utc.to_rfc3339(),
        is_valid,
    }))
}

/// Accept an invitation.
///
/// POST /invitations/{token}/accept
#[tracing::instrument(skip_all)]
pub async fn accept_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(req): Json<AcceptByTokenRequest>,
) -> Result<(StatusCode, Json<AcceptInvitationResponse>), AppError> {
    let token_hash = hash_token(&token);

    // Find invitation
    let invitation = state
        .db
        .find_invitation_by_token_hash(&token_hash)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| {
            AppError::NotFound(anyhow::anyhow!("Invitation not found or already used"))
        })?;

    // Check if valid
    if !invitation.is_valid() {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Invitation has expired or already been accepted"
        )));
    }

    // Validate password strength (basic check)
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Password must be at least 8 characters"
        )));
    }

    // Create user
    let user = User::new(
        invitation.tenant_id,
        invitation.email.clone(),
        req.display_name,
    );
    let user_id = user.user_id;

    state
        .db
        .insert_user(&user)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Create password identity
    let password_hash = hash_password(&req.password)?;
    let identity = UserIdentity::new_password(user_id, password_hash);

    state
        .db
        .insert_user_identity(&identity)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Create org assignment
    let assignment = OrgAssignment::new(
        invitation.tenant_id,
        user_id,
        invitation.org_node_id,
        invitation.role_id,
    );

    state
        .db
        .insert_org_assignment(&assignment)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Mark invitation as accepted
    state
        .db
        .accept_invitation(invitation.invitation_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Generate tokens
    let (access_token, refresh_token, refresh_token_id) = state
        .jwt
        .generate_token_pair(
            &user_id.to_string(),
            &invitation.tenant_id.to_string(),
            "",
            &invitation.email,
        )
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e)))?;

    // Store refresh session
    let refresh_hash = hash_token(&refresh_token_id);
    let session = RefreshSession::new(user_id, refresh_hash, state.jwt.refresh_token_expiry_days());
    state
        .db
        .insert_refresh_session(&session)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    tracing::info!(
        user_id = %user_id,
        invitation_id = %invitation.invitation_id,
        "Invitation accepted"
    );

    Ok((
        StatusCode::CREATED,
        Json(AcceptInvitationResponse {
            user_id,
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt.access_token_expiry_seconds(),
        }),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Hash a token for storage.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Password hashing failed: {}", e)))
}
