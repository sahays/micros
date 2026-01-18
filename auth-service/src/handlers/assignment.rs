//! Org assignment handlers for auth-service v2.
//!
//! Implements time-bounded immutable org assignments.
//! Key principles:
//! - Assignments are immutable (never update, just end and create new)
//! - Time-bounded with start_utc and end_utc
//! - Links user + role + org node

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::handlers::auth::MessageResponse;
use crate::models::OrgAssignment;
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to create an org assignment.
#[derive(Debug, Deserialize)]
pub struct CreateAssignmentRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub org_node_id: Uuid,
    pub start_utc: Option<DateTime<Utc>>,
}

/// Org assignment response.
#[derive(Debug, Serialize)]
pub struct AssignmentResponse {
    pub assignment_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub org_node_id: Uuid,
    pub start_utc: DateTime<Utc>,
    pub end_utc: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl From<OrgAssignment> for AssignmentResponse {
    fn from(a: OrgAssignment) -> Self {
        let is_active = a.is_active();
        Self {
            assignment_id: a.assignment_id,
            tenant_id: a.tenant_id,
            user_id: a.user_id,
            role_id: a.role_id,
            org_node_id: a.org_node_id,
            start_utc: a.start_utc,
            end_utc: a.end_utc,
            is_active,
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new org assignment.
///
/// POST /assignments
pub async fn create_assignment(
    State(state): State<AppState>,
    Json(req): Json<CreateAssignmentRequest>,
) -> Result<(StatusCode, Json<AssignmentResponse>), AppError> {
    // Verify user exists
    state
        .db
        .find_user_by_id(req.user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    // Verify role exists
    let role = state
        .db
        .find_role_by_id(req.role_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Role not found")))?;

    // Verify org node exists
    let org_node = state
        .db
        .find_org_node_by_id(req.org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Org node not found")))?;

    // Verify role and org node are in the same tenant
    if role.tenant_id != org_node.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Role and org node must belong to the same tenant"
        )));
    }

    // Verify request tenant matches
    if role.tenant_id != req.tenant_id {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Tenant ID does not match role's tenant"
        )));
    }

    // Create assignment
    let assignment = if let Some(start_utc) = req.start_utc {
        OrgAssignment::new_with_start(
            req.tenant_id,
            req.user_id,
            req.org_node_id,
            req.role_id,
            start_utc,
        )
    } else {
        OrgAssignment::new(req.tenant_id, req.user_id, req.org_node_id, req.role_id)
    };

    state
        .db
        .insert_org_assignment(&assignment)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(AssignmentResponse::from(assignment)),
    ))
}

/// List active assignments for a user.
///
/// GET /users/:user_id/assignments
pub async fn list_user_assignments(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<AssignmentResponse>>, AppError> {
    // Verify user exists
    state
        .db
        .find_user_by_id(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    let assignments = state
        .db
        .find_active_assignments_for_user(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let responses: Vec<AssignmentResponse> = assignments
        .into_iter()
        .map(AssignmentResponse::from)
        .collect();

    Ok(Json(responses))
}

/// End an assignment (set end_utc to now).
/// Assignments are immutable - we only set end_utc, never delete.
///
/// POST /assignments/:assignment_id/end
pub async fn end_assignment(
    State(state): State<AppState>,
    Path(assignment_id): Path<Uuid>,
) -> Result<Json<MessageResponse>, AppError> {
    state
        .db
        .end_assignment(assignment_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok(Json(MessageResponse {
        message: "Assignment ended successfully".to_string(),
    }))
}
