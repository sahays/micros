//! Role and capability handlers for auth-service v2.
//!
//! Implements role management and capability assignment.
//! Key principle: never authorize by role label, only by capability key.

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Capability, Role};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to create a new role.
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub tenant_id: Uuid,
    pub role_label: String,
}

/// Request to assign capability to role.
#[derive(Debug, Deserialize)]
pub struct AssignCapabilityRequest {
    pub capability_key: String,
}

/// Role response.
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub role_id: Uuid,
    pub tenant_id: Uuid,
    pub role_label: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            role_id: role.role_id,
            tenant_id: role.tenant_id,
            role_label: role.role_label,
            created_utc: role.created_utc,
        }
    }
}

/// Role with capabilities response.
#[derive(Debug, Serialize)]
pub struct RoleWithCapabilitiesResponse {
    #[serde(flatten)]
    pub role: RoleResponse,
    pub capabilities: Vec<String>,
}

/// Capability response.
#[derive(Debug, Serialize)]
pub struct CapabilityResponse {
    pub cap_id: Uuid,
    pub cap_key: String,
    pub created_utc: DateTime<Utc>,
}

impl From<Capability> for CapabilityResponse {
    fn from(cap: Capability) -> Self {
        Self {
            cap_id: cap.cap_id,
            cap_key: cap.cap_key,
            created_utc: cap.created_utc,
        }
    }
}

// Re-use MessageResponse from auth module
use crate::handlers::auth::MessageResponse;

// ============================================================================
// Role Handlers
// ============================================================================

/// Create a new role.
///
/// POST /roles
pub async fn create_role(
    State(state): State<AppState>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<RoleResponse>), AppError> {
    // Verify tenant exists
    let tenant = state
        .db
        .find_tenant_by_id(req.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    if !tenant.is_active() {
        return Err(AppError::BadRequest(anyhow::anyhow!("Tenant is suspended")));
    }

    // Create role
    let role = Role::new(req.tenant_id, req.role_label);

    state
        .db
        .insert_role(&role)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((StatusCode::CREATED, Json(RoleResponse::from(role))))
}

/// Get role by ID.
///
/// GET /roles/:role_id
pub async fn get_role(
    State(state): State<AppState>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<RoleWithCapabilitiesResponse>, AppError> {
    let role = state
        .db
        .find_role_by_id(role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Role not found")))?;

    let capabilities = state
        .db
        .get_role_capabilities(role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(RoleWithCapabilitiesResponse {
        role: RoleResponse::from(role),
        capabilities,
    }))
}

/// List roles for a tenant.
///
/// GET /tenants/:tenant_id/roles
pub async fn list_tenant_roles(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<RoleResponse>>, AppError> {
    // Verify tenant exists
    state
        .db
        .find_tenant_by_id(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    let roles = state
        .db
        .find_roles_by_tenant(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let responses: Vec<RoleResponse> = roles.into_iter().map(RoleResponse::from).collect();

    Ok(Json(responses))
}

/// Assign capability to role.
///
/// POST /roles/:role_id/capabilities
pub async fn assign_capability(
    State(state): State<AppState>,
    Path(role_id): Path<Uuid>,
    Json(req): Json<AssignCapabilityRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    // Verify role exists
    state
        .db
        .find_role_by_id(role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Role not found")))?;

    // Verify capability exists
    let capability = state
        .db
        .find_capability_by_key(&req.capability_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Capability not found")))?;

    // Assign capability to role
    state
        .db
        .assign_capability_to_role(role_id, capability.cap_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(MessageResponse {
        message: format!("Capability '{}' assigned to role", req.capability_key),
    }))
}

/// Get capabilities for a role.
///
/// GET /roles/:role_id/capabilities
pub async fn get_role_capabilities(
    State(state): State<AppState>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    // Verify role exists
    state
        .db
        .find_role_by_id(role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Role not found")))?;

    let capabilities = state
        .db
        .get_role_capabilities(role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(capabilities))
}

// ============================================================================
// Capability Handlers
// ============================================================================

/// List all capabilities.
///
/// GET /capabilities
pub async fn list_capabilities(
    State(state): State<AppState>,
) -> Result<Json<Vec<CapabilityResponse>>, AppError> {
    let capabilities = state
        .db
        .get_all_capabilities()
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let responses: Vec<CapabilityResponse> = capabilities
        .into_iter()
        .map(CapabilityResponse::from)
        .collect();

    Ok(Json(responses))
}

/// Get capability by key.
///
/// GET /capabilities/:cap_key
pub async fn get_capability(
    State(state): State<AppState>,
    Path(cap_key): Path<String>,
) -> Result<Json<CapabilityResponse>, AppError> {
    let capability = state
        .db
        .find_capability_by_key(&cap_key)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Capability not found")))?;

    Ok(Json(CapabilityResponse::from(capability)))
}
