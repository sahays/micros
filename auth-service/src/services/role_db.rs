//! Role and capability database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::{Capability, Role};
use crate::services::database::Database;

impl Database {
    // ==================== Role Operations ====================

    /// Find role by ID.
    pub async fn find_role_by_id(&self, role_id: Uuid) -> Result<Option<Role>, AppError> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE role_id = $1")
            .bind(role_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find all roles for a tenant.
    pub async fn find_roles_by_tenant(&self, tenant_id: Uuid) -> Result<Vec<Role>, AppError> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE tenant_id = $1 ORDER BY role_label")
            .bind(tenant_id)
            .fetch_all(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new role.
    pub async fn insert_role(&self, role: &Role) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO roles (role_id, tenant_id, role_label, created_utc)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(role.role_id)
        .bind(role.tenant_id)
        .bind(&role.role_label)
        .bind(role.created_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Get capabilities for a role.
    pub async fn get_role_capabilities(&self, role_id: Uuid) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.cap_key FROM capabilities c
            JOIN role_capabilities rc ON c.cap_id = rc.cap_id
            WHERE rc.role_id = $1
            "#,
        )
        .bind(role_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    /// Assign capability to role.
    pub async fn assign_capability_to_role(
        &self,
        role_id: Uuid,
        cap_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO role_capabilities (role_id, cap_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(role_id)
        .bind(cap_id)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Delete a role by ID.
    pub async fn delete_role(&self, role_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM roles WHERE role_id = $1")
            .bind(role_id)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a capability from a role.
    pub async fn revoke_capability_from_role(
        &self,
        role_id: Uuid,
        cap_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM role_capabilities WHERE role_id = $1 AND cap_id = $2")
            .bind(role_id)
            .bind(cap_id)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Capability Operations ====================

    /// Find capability by ID.
    pub async fn find_capability_by_id(
        &self,
        cap_id: Uuid,
    ) -> Result<Option<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE cap_id = $1")
            .bind(cap_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find capability by key.
    pub async fn find_capability_by_key(
        &self,
        cap_key: &str,
    ) -> Result<Option<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE cap_key = $1")
            .bind(cap_key)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Get all capabilities.
    pub async fn get_all_capabilities(&self) -> Result<Vec<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities ORDER BY cap_key")
            .fetch_all(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new capability.
    pub async fn insert_capability(&self, cap: &Capability) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO capabilities (cap_id, cap_key, created_utc)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(cap.cap_id)
        .bind(&cap.cap_key)
        .bind(cap.created_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
