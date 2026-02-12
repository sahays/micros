//! Org assignment and visibility grant database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::{OrgAssignment, VisibilityGrant};
use crate::services::database::Database;

impl Database {
    // ==================== Org Assignment Operations ====================

    /// Find active assignments for a user.
    pub async fn find_active_assignments_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OrgAssignment>, AppError> {
        sqlx::query_as::<_, OrgAssignment>(
            r#"
            SELECT * FROM org_assignments
            WHERE user_id = $1
            AND start_utc <= NOW()
            AND (end_utc IS NULL OR end_utc > NOW())
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new org assignment.
    pub async fn insert_org_assignment(&self, assignment: &OrgAssignment) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO org_assignments (assignment_id, tenant_id, user_id, org_node_id, role_id, start_utc, end_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(assignment.assignment_id)
        .bind(assignment.tenant_id)
        .bind(assignment.user_id)
        .bind(assignment.org_node_id)
        .bind(assignment.role_id)
        .bind(assignment.start_utc)
        .bind(assignment.end_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// End an assignment (set end_utc).
    pub async fn end_assignment(&self, assignment_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE org_assignments SET end_utc = NOW() WHERE assignment_id = $1")
            .bind(assignment_id)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Visibility Grant Operations ====================

    /// Find visibility grants for a user.
    pub async fn find_visibility_grants_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>("SELECT * FROM visibility_grants WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find active visibility grants for a user (within time bounds).
    pub async fn find_active_visibility_grants_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>(
            r#"
            SELECT * FROM visibility_grants
            WHERE user_id = $1
              AND start_utc <= NOW()
              AND (end_utc IS NULL OR end_utc > NOW())
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find visibility grant by ID.
    pub async fn find_visibility_grant_by_id(
        &self,
        grant_id: Uuid,
    ) -> Result<Option<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>("SELECT * FROM visibility_grants WHERE grant_id = $1")
            .bind(grant_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a visibility grant.
    pub async fn insert_visibility_grant(&self, grant: &VisibilityGrant) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO visibility_grants (grant_id, tenant_id, user_id, org_node_id, access_scope_code, start_utc, end_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(grant.grant_id)
        .bind(grant.tenant_id)
        .bind(grant.user_id)
        .bind(grant.org_node_id)
        .bind(&grant.access_scope_code)
        .bind(grant.start_utc)
        .bind(grant.end_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a visibility grant by setting end_utc to now.
    pub async fn revoke_visibility_grant(&self, grant_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE visibility_grants SET end_utc = NOW() WHERE grant_id = $1 AND (end_utc IS NULL OR end_utc > NOW())")
            .bind(grant_id)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
