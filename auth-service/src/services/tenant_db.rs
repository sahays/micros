//! Tenant database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::Tenant;
use crate::services::database::Database;

impl Database {
    // ==================== Tenant Operations ====================

    /// Find tenant by ID.
    pub async fn find_tenant_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find tenant by slug.
    pub async fn find_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_slug = $1")
            .bind(slug)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new tenant.
    pub async fn insert_tenant(&self, tenant: &Tenant) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, tenant_slug, tenant_label, tenant_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.tenant_slug)
        .bind(&tenant.tenant_label)
        .bind(&tenant.tenant_state_code)
        .bind(tenant.created_utc)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Count total number of tenants.
    pub async fn count_tenants(&self) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
            .fetch_one(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(row.0)
    }
}
