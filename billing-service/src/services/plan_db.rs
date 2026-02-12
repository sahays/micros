//! Plan and usage component database operations.
#![allow(clippy::too_many_arguments)]

use crate::models::{
    BillingPlan, CreatePlan, CreateUsageComponent, ListPlansFilter, UpdatePlan, UsageComponent,
};
use crate::services::database::Database;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Plan Operations
    // =========================================================================

    /// Create a new billing plan.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_plan(&self, input: &CreatePlan) -> Result<BillingPlan, AppError> {
        let plan_id = Uuid::new_v4();
        let plan = sqlx::query_as::<_, BillingPlan>(
            r#"
            INSERT INTO billing_plans (plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
            "#,
        )
        .bind(plan_id)
        .bind(input.tenant_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.billing_interval.as_str())
        .bind(input.interval_count)
        .bind(input.base_price)
        .bind(&input.currency)
        .bind(input.tax_rate_id)
        .bind(&input.metadata)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create plan: {}", e)))?;

        info!(plan_id = %plan.plan_id, name = %plan.name, "Plan created");

        Ok(plan)
    }

    /// Create a usage component for a plan.
    #[instrument(skip(self, input), fields(plan_id = %input.plan_id))]
    pub async fn create_usage_component(
        &self,
        input: &CreateUsageComponent,
    ) -> Result<UsageComponent, AppError> {
        let component_id = Uuid::new_v4();
        let component = sqlx::query_as::<_, UsageComponent>(
            r#"
            INSERT INTO usage_components (component_id, plan_id, name, unit_name, unit_price, included_units)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING component_id, plan_id, name, unit_name, unit_price, included_units, is_active, created_utc
            "#,
        )
        .bind(component_id)
        .bind(input.plan_id)
        .bind(&input.name)
        .bind(&input.unit_name)
        .bind(input.unit_price)
        .bind(input.included_units)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create usage component: {}", e)))?;

        Ok(component)
    }

    /// Get a plan by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, plan_id = %plan_id))]
    pub async fn get_plan(
        &self,
        tenant_id: Uuid,
        plan_id: Uuid,
    ) -> Result<Option<BillingPlan>, AppError> {
        let plan = sqlx::query_as::<_, BillingPlan>(
            r#"
            SELECT plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
            FROM billing_plans
            WHERE tenant_id = $1 AND plan_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(plan_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get plan: {}", e)))?;

        Ok(plan)
    }

    /// Get usage components for a plan.
    #[instrument(skip(self), fields(plan_id = %plan_id))]
    pub async fn get_usage_components(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<UsageComponent>, AppError> {
        let components = sqlx::query_as::<_, UsageComponent>(
            r#"
            SELECT component_id, plan_id, name, unit_name, unit_price, included_units, is_active, created_utc
            FROM usage_components
            WHERE plan_id = $1 AND is_active = TRUE
            ORDER BY name
            "#,
        )
        .bind(plan_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get usage components: {}", e)))?;

        Ok(components)
    }

    /// List plans for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_plans(
        &self,
        tenant_id: Uuid,
        filter: &ListPlansFilter,
    ) -> Result<Vec<BillingPlan>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;

        let plans = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, BillingPlan>(
                r#"
                SELECT plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
                FROM billing_plans
                WHERE tenant_id = $1
                  AND ($2::bool = TRUE OR is_archived = FALSE)
                  AND plan_id > $3
                ORDER BY plan_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(filter.include_archived)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BillingPlan>(
                r#"
                SELECT plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
                FROM billing_plans
                WHERE tenant_id = $1
                  AND ($2::bool = TRUE OR is_archived = FALSE)
                ORDER BY plan_id
                LIMIT $3
                "#,
            )
            .bind(tenant_id)
            .bind(filter.include_archived)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list plans: {}", e)))?;

        Ok(plans)
    }

    /// Update a plan.
    #[instrument(skip(self, input), fields(tenant_id = %tenant_id, plan_id = %plan_id))]
    pub async fn update_plan(
        &self,
        tenant_id: Uuid,
        plan_id: Uuid,
        input: &UpdatePlan,
    ) -> Result<Option<BillingPlan>, AppError> {
        let plan = sqlx::query_as::<_, BillingPlan>(
            r#"
            UPDATE billing_plans
            SET name = COALESCE($3, name),
                description = COALESCE($4, description),
                base_price = COALESCE($5, base_price),
                tax_rate_id = COALESCE($6, tax_rate_id),
                metadata = COALESCE($7, metadata)
            WHERE tenant_id = $1 AND plan_id = $2 AND is_archived = FALSE
            RETURNING plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
            "#,
        )
        .bind(tenant_id)
        .bind(plan_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.base_price)
        .bind(input.tax_rate_id)
        .bind(&input.metadata)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update plan: {}", e)))?;

        Ok(plan)
    }

    /// Archive a plan.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, plan_id = %plan_id))]
    pub async fn archive_plan(
        &self,
        tenant_id: Uuid,
        plan_id: Uuid,
    ) -> Result<Option<BillingPlan>, AppError> {
        let plan = sqlx::query_as::<_, BillingPlan>(
            r#"
            UPDATE billing_plans
            SET is_archived = TRUE, is_active = FALSE
            WHERE tenant_id = $1 AND plan_id = $2 AND is_archived = FALSE
            RETURNING plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
            "#,
        )
        .bind(tenant_id)
        .bind(plan_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to archive plan: {}", e)))?;

        if let Some(ref p) = plan {
            info!(plan_id = %p.plan_id, "Plan archived");
        }

        Ok(plan)
    }
}
