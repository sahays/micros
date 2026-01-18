//! Organization node handlers for auth-service v2.
//!
//! Implements org hierarchy management with closure table pattern.

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::OrgNode;
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to create a new org node.
#[derive(Debug, Deserialize)]
pub struct CreateOrgNodeRequest {
    pub tenant_id: Uuid,
    pub parent_org_node_id: Option<Uuid>,
    pub node_type_code: String,
    pub node_label: String,
}

/// Request to update an org node.
#[derive(Debug, Deserialize)]
pub struct UpdateOrgNodeRequest {
    pub org_node_label: Option<String>,
}

/// Org node response.
#[derive(Debug, Serialize)]
pub struct OrgNodeResponse {
    pub org_node_id: Uuid,
    pub tenant_id: Uuid,
    pub parent_org_node_id: Option<Uuid>,
    pub node_type_code: String,
    pub node_label: String,
    pub active_flag: bool,
    pub created_utc: DateTime<Utc>,
}

impl From<OrgNode> for OrgNodeResponse {
    fn from(node: OrgNode) -> Self {
        Self {
            org_node_id: node.org_node_id,
            tenant_id: node.tenant_id,
            parent_org_node_id: node.parent_org_node_id,
            node_type_code: node.node_type_code,
            node_label: node.node_label,
            active_flag: node.active_flag,
            created_utc: node.created_utc,
        }
    }
}

/// Org node tree response (with children).
#[derive(Debug, Serialize)]
pub struct OrgNodeTreeResponse {
    #[serde(flatten)]
    pub node: OrgNodeResponse,
    pub children: Vec<OrgNodeTreeResponse>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new org node.
///
/// POST /orgs
pub async fn create_org_node(
    State(state): State<AppState>,
    Json(req): Json<CreateOrgNodeRequest>,
) -> Result<(StatusCode, Json<OrgNodeResponse>), AppError> {
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

    // If parent is specified, verify it exists and belongs to same tenant
    if let Some(parent_id) = req.parent_org_node_id {
        let parent = state
            .db
            .find_org_node_by_id(parent_id)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Parent org node not found")))?;

        if parent.tenant_id != req.tenant_id {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Parent org node belongs to different tenant"
            )));
        }
    }

    // Create org node
    let node = OrgNode::new(
        req.tenant_id,
        req.node_type_code,
        req.node_label,
        req.parent_org_node_id,
    );

    state
        .db
        .insert_org_node(&node)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    Ok((StatusCode::CREATED, Json(OrgNodeResponse::from(node))))
}

/// Get org node by ID.
///
/// GET /orgs/:org_node_id
pub async fn get_org_node(
    State(state): State<AppState>,
    Path(org_node_id): Path<Uuid>,
) -> Result<Json<OrgNodeResponse>, AppError> {
    let node = state
        .db
        .find_org_node_by_id(org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Org node not found")))?;

    Ok(Json(OrgNodeResponse::from(node)))
}

/// List org nodes for a tenant.
///
/// GET /tenants/:tenant_id/orgs
pub async fn list_tenant_org_nodes(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<OrgNodeResponse>>, AppError> {
    // Verify tenant exists
    state
        .db
        .find_tenant_by_id(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    let nodes = state
        .db
        .find_org_nodes_by_tenant(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let responses: Vec<OrgNodeResponse> = nodes.into_iter().map(OrgNodeResponse::from).collect();

    Ok(Json(responses))
}

/// Get descendants of an org node (using closure table).
///
/// GET /orgs/:org_node_id/descendants
pub async fn get_org_node_descendants(
    State(state): State<AppState>,
    Path(org_node_id): Path<Uuid>,
) -> Result<Json<Vec<OrgNodeResponse>>, AppError> {
    // Verify org node exists
    state
        .db
        .find_org_node_by_id(org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Org node not found")))?;

    let descendants = state
        .db
        .find_org_node_descendants(org_node_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let responses: Vec<OrgNodeResponse> =
        descendants.into_iter().map(OrgNodeResponse::from).collect();

    Ok(Json(responses))
}

/// Build tree structure from flat list of org nodes.
///
/// GET /tenants/:tenant_id/orgs/tree
pub async fn get_tenant_org_tree(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<OrgNodeTreeResponse>>, AppError> {
    // Verify tenant exists
    state
        .db
        .find_tenant_by_id(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    let nodes = state
        .db
        .find_org_nodes_by_tenant(tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let tree = build_org_tree(nodes);

    Ok(Json(tree))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build a tree structure from a flat list of org nodes.
fn build_org_tree(nodes: Vec<OrgNode>) -> Vec<OrgNodeTreeResponse> {
    use std::collections::HashMap;

    // Create a map of node_id -> children
    let mut children_map: HashMap<Uuid, Vec<OrgNode>> = HashMap::new();
    let mut root_nodes: Vec<OrgNode> = Vec::new();

    for node in nodes {
        if let Some(parent_id) = node.parent_org_node_id {
            children_map.entry(parent_id).or_default().push(node);
        } else {
            root_nodes.push(node);
        }
    }

    // Recursively build tree
    fn build_subtree(
        node: OrgNode,
        children_map: &HashMap<Uuid, Vec<OrgNode>>,
    ) -> OrgNodeTreeResponse {
        let node_id = node.org_node_id;
        let children = children_map
            .get(&node_id)
            .map(|children| {
                children
                    .iter()
                    .cloned()
                    .map(|child| build_subtree(child, children_map))
                    .collect()
            })
            .unwrap_or_default();

        OrgNodeTreeResponse {
            node: OrgNodeResponse::from(node),
            children,
        }
    }

    root_nodes
        .into_iter()
        .map(|node| build_subtree(node, &children_map))
        .collect()
}
