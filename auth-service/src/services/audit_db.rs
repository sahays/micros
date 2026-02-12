//! Audit event database operations.
#![allow(clippy::too_many_arguments)]

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::AuditEvent;
use crate::services::database::Database;

impl Database {
    // ==================== Audit Event Operations ====================

    /// Insert an audit event.
    pub async fn insert_audit_event(&self, event: &AuditEvent) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO audit_events (event_id, tenant_id, actor_user_id, actor_svc_id, event_type_code, target_type, target_id, event_data, ip_address, user_agent, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(event.event_id)
        .bind(event.tenant_id)
        .bind(event.actor_user_id)
        .bind(event.actor_svc_id)
        .bind(&event.event_type_code)
        .bind(&event.target_type)
        .bind(event.target_id)
        .bind(&event.event_data)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(event.created_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find audit events with filtering and pagination.
    pub async fn find_audit_events(
        &self,
        tenant_id: Uuid,
        actor_user_id: Option<Uuid>,
        action_key: Option<&str>,
        entity_kind: Option<&str>,
        entity_id: Option<Uuid>,
        from_utc: Option<chrono::DateTime<chrono::Utc>>,
        to_utc: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditEvent>, i64), AppError> {
        // Build dynamic WHERE clause
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut param_idx = 2;

        if actor_user_id.is_some() {
            conditions.push(format!("actor_user_id = ${}", param_idx));
            param_idx += 1;
        }
        if action_key.is_some() {
            conditions.push(format!("event_type_code = ${}", param_idx));
            param_idx += 1;
        }
        if entity_kind.is_some() {
            conditions.push(format!("target_type = ${}", param_idx));
            param_idx += 1;
        }
        if entity_id.is_some() {
            conditions.push(format!("target_id = ${}", param_idx));
            param_idx += 1;
        }
        if from_utc.is_some() {
            conditions.push(format!("created_utc >= ${}", param_idx));
            param_idx += 1;
        }
        if to_utc.is_some() {
            conditions.push(format!("created_utc <= ${}", param_idx));
            param_idx += 1;
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_query = format!("SELECT COUNT(*) FROM audit_events WHERE {}", where_clause);

        // Data query
        let data_query =
            format!(
            "SELECT * FROM audit_events WHERE {} ORDER BY created_utc DESC LIMIT ${} OFFSET ${}",
            where_clause, param_idx, param_idx + 1
        );

        // Build and execute count query
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_query).bind(tenant_id);
        if let Some(user_id) = actor_user_id {
            count_q = count_q.bind(user_id);
        }
        if let Some(action) = action_key {
            count_q = count_q.bind(action);
        }
        if let Some(kind) = entity_kind {
            count_q = count_q.bind(kind);
        }
        if let Some(eid) = entity_id {
            count_q = count_q.bind(eid);
        }
        if let Some(from) = from_utc {
            count_q = count_q.bind(from);
        }
        if let Some(to) = to_utc {
            count_q = count_q.bind(to);
        }

        let (total,) = count_q
            .fetch_one(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // Build and execute data query
        let mut data_q = sqlx::query_as::<_, AuditEvent>(&data_query).bind(tenant_id);
        if let Some(user_id) = actor_user_id {
            data_q = data_q.bind(user_id);
        }
        if let Some(action) = action_key {
            data_q = data_q.bind(action);
        }
        if let Some(kind) = entity_kind {
            data_q = data_q.bind(kind);
        }
        if let Some(eid) = entity_id {
            data_q = data_q.bind(eid);
        }
        if let Some(from) = from_utc {
            data_q = data_q.bind(from);
        }
        if let Some(to) = to_utc {
            data_q = data_q.bind(to);
        }
        data_q = data_q.bind(limit).bind(offset);

        let events = data_q
            .fetch_all(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok((events, total))
    }
}
