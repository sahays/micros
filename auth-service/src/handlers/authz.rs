//! Authorization evaluation handlers for auth-service v2.
//!
//! Implements the authz/evaluate endpoint which:
//! - Evaluates authorization decisions
//! - Supports batch capability checks
//! - Returns detailed decision info

use axum::{
    extract::{Json, State},
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

/// Authorization evaluation request.
#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    /// The org node to check permissions at
    pub org_node_id: Uuid,
    /// Capabilities to evaluate
    pub capabilities: Vec<String>,
    /// Optional resource ID for "own" scope checks
    pub resource_owner_id: Option<Uuid>,
}

/// Single capability decision.
#[derive(Debug, Serialize)]
pub struct CapabilityDecision {
    pub capability: String,
    pub allowed: bool,
    pub reason: String,
    /// The assignment that granted this capability (if allowed)
    pub granted_by_assignment: Option<Uuid>,
    /// The org node where the capability was granted
    pub granted_at_org: Option<Uuid>,
}

/// Authorization evaluation response.
#[derive(Debug, Serialize)]
pub struct EvaluateResponse {
    pub user_id: Uuid,
    pub org_node_id: Uuid,
    pub all_allowed: bool,
    pub decisions: Vec<CapabilityDecision>,
}

/// Batch evaluation request.
#[derive(Debug, Deserialize)]
pub struct BatchEvaluateRequest {
    pub checks: Vec<EvaluateRequest>,
}

/// Batch evaluation response.
#[derive(Debug, Serialize)]
pub struct BatchEvaluateResponse {
    pub results: Vec<EvaluateResponse>,
    pub all_allowed: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// Evaluate authorization for capabilities at an org node.
///
/// POST /authz/evaluate
pub async fn evaluate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EvaluateRequest>,
) -> Result<Json<EvaluateResponse>, AppError> {
    // Extract user info from Authorization header
    let user_id = extract_user_id(&state, &headers)?;

    // Get active assignments for user
    let assignments = state
        .db
        .find_active_assignments_for_user(user_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Get all org nodes from root to target (ancestors)
    // For simplicity, we'll get the target node and check if any assignment covers it
    // A full implementation would use the closure table to get all ancestors

    // Build set of org nodes covered by user's assignments
    // (including subtree descendants for subtree-scoped capabilities)
    let mut covered_orgs: HashSet<Uuid> = HashSet::new();
    let mut org_caps: Vec<(Uuid, Uuid, Vec<String>)> = Vec::new(); // (assignment_id, org_node_id, caps)

    for assignment in &assignments {
        // Get capabilities for this role
        let caps = state
            .db
            .get_role_capabilities(assignment.role_id)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

        covered_orgs.insert(assignment.org_node_id);

        // If any capability has subtree scope, add descendants
        let has_subtree = caps.iter().any(|c| c.ends_with(":subtree"));

        org_caps.push((assignment.assignment_id, assignment.org_node_id, caps));
        if has_subtree {
            let descendants = state
                .db
                .find_org_node_descendants(assignment.org_node_id)
                .await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

            for desc in descendants {
                covered_orgs.insert(desc.org_node_id);
            }
        }
    }

    // Evaluate each capability
    let mut decisions: Vec<CapabilityDecision> = Vec::new();

    for cap_key in &req.capabilities {
        let decision = evaluate_capability(
            cap_key,
            req.org_node_id,
            &org_caps,
            &covered_orgs,
            user_id,
            req.resource_owner_id,
        );
        decisions.push(decision);
    }

    let all_allowed = decisions.iter().all(|d| d.allowed);

    Ok(Json(EvaluateResponse {
        user_id,
        org_node_id: req.org_node_id,
        all_allowed,
        decisions,
    }))
}

/// Batch evaluate multiple authorization requests.
///
/// POST /authz/batch-evaluate
pub async fn batch_evaluate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BatchEvaluateRequest>,
) -> Result<Json<BatchEvaluateResponse>, AppError> {
    let mut results: Vec<EvaluateResponse> = Vec::new();

    for check in req.checks {
        let json_result = evaluate(State(state.clone()), headers.clone(), Json(check)).await?;
        results.push(json_result.0);
    }

    let all_allowed = results.iter().all(|r| r.all_allowed);

    Ok(Json(BatchEvaluateResponse {
        results,
        all_allowed,
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract user ID from Authorization header.
fn extract_user_id(state: &AppState, headers: &HeaderMap) -> Result<Uuid, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Missing authorization header")))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid authorization header")))?;

    let claims = state
        .jwt
        .validate_access_token(token)
        .map_err(|e| AppError::AuthError(anyhow::anyhow!("Invalid token: {}", e)))?;

    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid user ID in token")))
}

/// Evaluate a single capability.
fn evaluate_capability(
    cap_key: &str,
    target_org: Uuid,
    org_caps: &[(Uuid, Uuid, Vec<String>)], // (assignment_id, org_node_id, caps)
    covered_orgs: &HashSet<Uuid>,
    user_id: Uuid,
    resource_owner_id: Option<Uuid>,
) -> CapabilityDecision {
    // Parse capability to check for scope
    let parts: Vec<&str> = cap_key.split(':').collect();
    let base_cap = if parts.len() >= 2 {
        format!("{}:{}", parts[0], parts[1])
    } else {
        cap_key.to_string()
    };
    let scope = parts.get(2).copied();

    // Check if target org is covered
    if !covered_orgs.contains(&target_org) {
        return CapabilityDecision {
            capability: cap_key.to_string(),
            allowed: false,
            reason: "No assignment covers target org node".to_string(),
            granted_by_assignment: None,
            granted_at_org: None,
        };
    }

    // Find matching capability in assignments
    for (assignment_id, org_node_id, caps) in org_caps {
        // Check for exact match
        if caps.contains(&cap_key.to_string()) {
            // Handle scope
            if let Some(s) = scope {
                match s {
                    "own" => {
                        // "own" scope requires resource owner to match user
                        if resource_owner_id == Some(user_id) {
                            return CapabilityDecision {
                                capability: cap_key.to_string(),
                                allowed: true,
                                reason: "Capability granted with own scope".to_string(),
                                granted_by_assignment: Some(*assignment_id),
                                granted_at_org: Some(*org_node_id),
                            };
                        } else {
                            return CapabilityDecision {
                                capability: cap_key.to_string(),
                                allowed: false,
                                reason: "Own scope requires resource ownership".to_string(),
                                granted_by_assignment: None,
                                granted_at_org: None,
                            };
                        }
                    }
                    "subtree" => {
                        // Subtree scope - already handled by covered_orgs expansion
                        return CapabilityDecision {
                            capability: cap_key.to_string(),
                            allowed: true,
                            reason: "Capability granted with subtree scope".to_string(),
                            granted_by_assignment: Some(*assignment_id),
                            granted_at_org: Some(*org_node_id),
                        };
                    }
                    _ => {
                        // Unknown scope - treat as regular capability
                    }
                }
            }

            return CapabilityDecision {
                capability: cap_key.to_string(),
                allowed: true,
                reason: "Capability granted".to_string(),
                granted_by_assignment: Some(*assignment_id),
                granted_at_org: Some(*org_node_id),
            };
        }

        // Check for base capability match (e.g., "crm.visit:view" matches "crm.visit:view:subtree")
        for cap in caps {
            if cap.starts_with(&base_cap) {
                let cap_parts: Vec<&str> = cap.split(':').collect();
                let cap_scope = cap_parts.get(2).copied();

                if cap_scope == Some("subtree") && *org_node_id == target_org {
                    return CapabilityDecision {
                        capability: cap_key.to_string(),
                        allowed: true,
                        reason: "Capability granted via subtree inheritance".to_string(),
                        granted_by_assignment: Some(*assignment_id),
                        granted_at_org: Some(*org_node_id),
                    };
                }
            }
        }
    }

    CapabilityDecision {
        capability: cap_key.to_string(),
        allowed: false,
        reason: "Capability not found in any assignment".to_string(),
        granted_by_assignment: None,
        granted_at_org: None,
    }
}
