//! Auth context handlers for auth-service v2.
//!
//! Implements the auth context endpoint which returns:
//! - User's capabilities at a given org node
//! - Effective permissions based on role assignments
//! - Subtree context for hierarchical queries

use axum::{
    extract::{Json, Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Query params for auth context.
#[derive(Debug, Deserialize)]
pub struct AuthContextQuery {
    /// Org node to get context for (optional, returns all if not specified)
    pub org_node_id: Option<Uuid>,
}

/// Auth context response.
#[derive(Debug, Serialize)]
pub struct AuthContextResponse {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub org_node_id: Option<Uuid>,
    pub capabilities: Vec<String>,
    pub assignments: Vec<AssignmentSummary>,
}

/// Summary of an assignment for context.
#[derive(Debug, Serialize)]
pub struct AssignmentSummary {
    pub assignment_id: Uuid,
    pub org_node_id: Uuid,
    pub role_id: Uuid,
    pub role_label: String,
    pub capabilities: Vec<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get auth context implementation - can be called from REST and gRPC handlers.
pub async fn get_auth_context_impl(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    org_node_id: Option<Uuid>,
) -> Result<AuthContextResponse, AppError> {
    // Get active assignments for user
    let assignments = state
        .db
        .find_active_assignments_for_user(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Filter by org node if specified
    let filtered_assignments = if let Some(org_node_id) = org_node_id {
        // Get descendants of the org node (including itself)
        let descendants: HashSet<Uuid> = state
            .db
            .find_org_node_descendants(org_node_id)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
            .into_iter()
            .map(|n| n.org_node_id)
            .collect();

        assignments
            .into_iter()
            .filter(|a| descendants.contains(&a.org_node_id) || a.org_node_id == org_node_id)
            .collect()
    } else {
        assignments
    };

    // Collect capabilities from all roles
    let mut all_capabilities: HashSet<String> = HashSet::new();
    let mut assignment_summaries: Vec<AssignmentSummary> = Vec::new();

    for assignment in filtered_assignments {
        // Get role info
        let role = state
            .db
            .find_role_by_id(assignment.role_id)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

        if let Some(role) = role {
            // Get capabilities for this role
            let caps = state
                .db
                .get_role_capabilities(assignment.role_id)
                .await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

            // Add to all capabilities
            for cap in &caps {
                all_capabilities.insert(cap.clone());
            }

            assignment_summaries.push(AssignmentSummary {
                assignment_id: assignment.assignment_id,
                org_node_id: assignment.org_node_id,
                role_id: assignment.role_id,
                role_label: role.role_label,
                capabilities: caps,
            });
        }
    }

    Ok(AuthContextResponse {
        user_id,
        tenant_id,
        org_node_id,
        capabilities: all_capabilities.into_iter().collect(),
        assignments: assignment_summaries,
    })
}

/// Get auth context for current user.
/// Returns capabilities at the specified org node (or all if not specified).
///
/// GET /auth/context
pub async fn get_auth_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuthContextQuery>,
) -> Result<Json<AuthContextResponse>, AppError> {
    // Extract user info from Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Missing authorization header")))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid authorization header")))?;

    // Validate token
    let claims = state
        .jwt
        .validate_access_token(token)
        .map_err(|e| AppError::AuthError(anyhow::anyhow!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid user ID in token")))?;

    let tenant_id = Uuid::parse_str(&claims.app_id)
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid tenant ID in token")))?;

    let response = get_auth_context_impl(&state, user_id, tenant_id, query.org_node_id).await?;
    Ok(Json(response))
}

/// Check if user has a specific capability at an org node.
///
/// GET /auth/check
#[derive(Debug, Deserialize)]
pub struct AuthCheckQuery {
    pub org_node_id: Uuid,
    pub capability: String,
}

#[derive(Debug, Serialize)]
pub struct AuthCheckResponse {
    pub allowed: bool,
    pub capability: String,
    pub org_node_id: Uuid,
    pub matched_assignment: Option<Uuid>,
}

/// Check capability implementation - can be called from REST and gRPC handlers.
pub async fn check_capability_impl(
    state: &AppState,
    user_id: Uuid,
    org_node_id: Uuid,
    capability: String,
) -> Result<AuthCheckResponse, AppError> {
    // Get active assignments for user
    let assignments = state
        .db
        .find_active_assignments_for_user(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Get descendants of the org node (including itself for inheritance)
    let ancestors_and_self: HashSet<Uuid> = {
        // For now, just check exact match and ancestors
        // A full implementation would check the closure table for ancestors
        let mut set = HashSet::new();
        set.insert(org_node_id);
        set
    };

    // Check if any assignment grants the capability
    for assignment in assignments {
        if ancestors_and_self.contains(&assignment.org_node_id) {
            // Get capabilities for this role
            let caps = state
                .db
                .get_role_capabilities(assignment.role_id)
                .await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

            if caps.contains(&capability) {
                return Ok(AuthCheckResponse {
                    allowed: true,
                    capability,
                    org_node_id,
                    matched_assignment: Some(assignment.assignment_id),
                });
            }
        }
    }

    Ok(AuthCheckResponse {
        allowed: false,
        capability,
        org_node_id,
        matched_assignment: None,
    })
}

pub async fn check_capability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuthCheckQuery>,
) -> Result<Json<AuthCheckResponse>, AppError> {
    // Extract user info from Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Missing authorization header")))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid authorization header")))?;

    // Validate token
    let claims = state
        .jwt
        .validate_access_token(token)
        .map_err(|e| AppError::AuthError(anyhow::anyhow!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid user ID in token")))?;

    let response =
        check_capability_impl(&state, user_id, query.org_node_id, query.capability).await?;
    Ok(Json(response))
}
