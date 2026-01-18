//! Visibility grant handlers for auth-service v2.
//!
//! Implements cross-org visibility grants:
//! - Grant creation with time bounds
//! - Grant revocation
//! - Active grants query

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::{CreateVisibilityGrantRequest, VisibilityGrant, VisibilityGrantResponse};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Query Parameters
// ============================================================================

/// Query params for listing grants.
#[derive(Debug, Deserialize)]
pub struct ListGrantsQuery {
    /// If true, only return active grants (within time bounds)
    pub active: Option<bool>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response after creating a grant.
#[derive(Debug, serde::Serialize)]
pub struct CreateGrantResponse {
    pub grant_id: Uuid,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new visibility grant.
///
/// POST /visibility-grants
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %req.tenant_id,
        user_id = %req.user_id,
        org_node_id = %req.org_node_id
    )
)]
pub async fn create_visibility_grant(
    State(state): State<AppState>,
    Json(req): Json<CreateVisibilityGrantRequest>,
) -> Result<(StatusCode, Json<CreateGrantResponse>), AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(req.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Validate user exists
    let user = state
        .db
        .find_user_by_id(req.user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    // Ensure user belongs to tenant
    if user.tenant_id != req.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "User does not belong to specified tenant"
        )));
    }

    // Validate org node exists
    let org_node = state
        .db
        .find_org_node_by_id(req.org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Org node not found")))?;

    // Ensure org node belongs to tenant
    if org_node.tenant_id != req.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Org node does not belong to specified tenant"
        )));
    }

    // Validate time bounds
    if let (Some(start), Some(end)) = (req.start_utc, req.end_utc) {
        if end <= start {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "end_utc must be after start_utc"
            )));
        }
    }

    // Create visibility grant
    let grant = VisibilityGrant::new(
        req.tenant_id,
        req.user_id,
        req.org_node_id,
        req.access_scope,
        req.start_utc,
        req.end_utc,
    );
    let grant_id = grant.grant_id;

    state
        .db
        .insert_visibility_grant(&grant)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    tracing::info!(
        grant_id = %grant_id,
        user_id = %req.user_id,
        org_node_id = %req.org_node_id,
        "Visibility grant created"
    );

    Ok((StatusCode::CREATED, Json(CreateGrantResponse { grant_id })))
}

/// Revoke a visibility grant.
///
/// POST /visibility-grants/{grant_id}/revoke
#[tracing::instrument(skip(state), fields(grant_id = %grant_id))]
pub async fn revoke_visibility_grant(
    State(state): State<AppState>,
    Path(grant_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Verify grant exists
    let grant = state
        .db
        .find_visibility_grant_by_id(grant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Visibility grant not found")))?;

    // Check if already revoked
    if !grant.is_active() {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Grant is already inactive or expired"
        )));
    }

    // Revoke the grant
    state
        .db
        .revoke_visibility_grant(grant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    tracing::info!(grant_id = %grant_id, "Visibility grant revoked");

    Ok(StatusCode::NO_CONTENT)
}

/// List visibility grants for a user.
///
/// GET /users/{user_id}/visibility-grants
#[tracing::instrument(skip(state), fields(user_id = %user_id, active_only = ?query.active))]
pub async fn list_user_visibility_grants(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<ListGrantsQuery>,
) -> Result<Json<Vec<VisibilityGrantResponse>>, AppError> {
    // Validate user exists
    let _user = state
        .db
        .find_user_by_id(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    // Get grants based on active filter
    let grants = if query.active.unwrap_or(false) {
        state
            .db
            .find_active_visibility_grants_for_user(user_id)
            .await
    } else {
        state.db.find_visibility_grants_for_user(user_id).await
    }
    .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let response: Vec<VisibilityGrantResponse> = grants
        .into_iter()
        .map(VisibilityGrantResponse::from)
        .collect();

    Ok(Json(response))
}
