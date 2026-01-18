//! Audit event handlers for auth-service v2.
//!
//! Provides query endpoint for audit events with filtering and pagination.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::AuditEventResponse;
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Query Parameters
// ============================================================================

/// Query params for listing audit events.
#[derive(Debug, Deserialize)]
pub struct ListAuditEventsQuery {
    pub tenant_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action_key: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_id: Option<Uuid>,
    pub from_utc: Option<DateTime<Utc>>,
    pub to_utc: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

// ============================================================================
// Response Types
// ============================================================================

/// Paginated audit events response.
#[derive(Debug, Serialize)]
pub struct AuditEventsResponse {
    pub events: Vec<AuditEventResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ============================================================================
// Handlers
// ============================================================================

/// List audit events with filtering and pagination.
///
/// GET /audit/events
#[tracing::instrument(
    skip(state),
    fields(
        tenant_id = %query.tenant_id,
        action_key = ?query.action_key,
        entity_kind = ?query.entity_kind,
        limit = query.limit,
        offset = query.offset
    )
)]
pub async fn list_audit_events(
    State(state): State<AppState>,
    Query(query): Query<ListAuditEventsQuery>,
) -> Result<Json<AuditEventsResponse>, AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(query.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Validate and clamp limits
    let limit = query.limit.clamp(1, 1000);
    let offset = query.offset.max(0);

    // Query events with filters
    let (events, total) = state
        .db
        .find_audit_events(
            query.tenant_id,
            query.actor_user_id,
            query.action_key.as_deref(),
            query.entity_kind.as_deref(),
            query.entity_id,
            query.from_utc,
            query.to_utc,
            limit,
            offset,
        )
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    let events: Vec<AuditEventResponse> =
        events.into_iter().map(AuditEventResponse::from).collect();

    Ok(Json(AuditEventsResponse {
        events,
        total,
        limit,
        offset,
    }))
}
