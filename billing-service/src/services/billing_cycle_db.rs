//! Billing cycle and charge database operations.
#![allow(clippy::too_many_arguments)]

use crate::models::{
    BillingCycle, BillingCycleStatus, BillingInterval, Charge, CreateCharge,
    ListBillingCyclesFilter, ListChargesFilter, SubscriptionStatus,
};
use crate::services::database::Database;
use crate::services::subscription_db::calculate_period_end;
use chrono::NaiveDate;
use service_core::error::AppError;
use tracing::instrument;
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Billing Cycle Operations
    // =========================================================================

    /// Create a billing cycle for a subscription.
    #[instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn create_billing_cycle(
        &self,
        subscription_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<BillingCycle, AppError> {
        let cycle_id = Uuid::new_v4();
        let cycle = sqlx::query_as::<_, BillingCycle>(
            r#"
            INSERT INTO billing_cycles (cycle_id, subscription_id, period_start, period_end)
            VALUES ($1, $2, $3, $4)
            RETURNING cycle_id, subscription_id, period_start, period_end, status, invoice_id, created_utc, updated_utc
            "#,
        )
        .bind(cycle_id)
        .bind(subscription_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing cycle: {}", e)))?;

        Ok(cycle)
    }

    /// Get a billing cycle by ID with tenant isolation via subscription.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, cycle_id = %cycle_id))]
    pub async fn get_billing_cycle(
        &self,
        tenant_id: Uuid,
        cycle_id: Uuid,
    ) -> Result<Option<BillingCycle>, AppError> {
        let cycle = sqlx::query_as::<_, BillingCycle>(
            r#"
            SELECT bc.cycle_id, bc.subscription_id, bc.period_start, bc.period_end, bc.status, bc.invoice_id, bc.created_utc, bc.updated_utc
            FROM billing_cycles bc
            JOIN subscriptions s ON bc.subscription_id = s.subscription_id
            WHERE s.tenant_id = $1 AND bc.cycle_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(cycle_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing cycle: {}", e)))?;

        Ok(cycle)
    }

    /// Get the current (pending) billing cycle for a subscription.
    #[instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn get_current_billing_cycle(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<BillingCycle>, AppError> {
        let cycle = sqlx::query_as::<_, BillingCycle>(
            r#"
            SELECT cycle_id, subscription_id, period_start, period_end, status, invoice_id, created_utc, updated_utc
            FROM billing_cycles
            WHERE subscription_id = $1 AND status = 'pending'
            ORDER BY period_start DESC
            LIMIT 1
            "#,
        )
        .bind(subscription_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get current billing cycle: {}", e)))?;

        Ok(cycle)
    }

    /// List billing cycles for a subscription.
    #[instrument(skip(self, filter), fields(subscription_id = %subscription_id))]
    pub async fn list_billing_cycles(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
        filter: &ListBillingCyclesFilter,
    ) -> Result<Vec<BillingCycle>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;
        let status_str = filter.status.map(|s| s.as_str().to_string());

        let cycles = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, BillingCycle>(
                r#"
                SELECT bc.cycle_id, bc.subscription_id, bc.period_start, bc.period_end, bc.status, bc.invoice_id, bc.created_utc, bc.updated_utc
                FROM billing_cycles bc
                JOIN subscriptions s ON bc.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND bc.subscription_id = $2
                  AND ($3::varchar IS NULL OR bc.status = $3)
                  AND bc.cycle_id > $4
                ORDER BY bc.cycle_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(subscription_id)
            .bind(&status_str)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BillingCycle>(
                r#"
                SELECT bc.cycle_id, bc.subscription_id, bc.period_start, bc.period_end, bc.status, bc.invoice_id, bc.created_utc, bc.updated_utc
                FROM billing_cycles bc
                JOIN subscriptions s ON bc.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND bc.subscription_id = $2
                  AND ($3::varchar IS NULL OR bc.status = $3)
                ORDER BY bc.cycle_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(subscription_id)
            .bind(&status_str)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list billing cycles: {}", e)))?;

        Ok(cycles)
    }

    /// Update billing cycle status.
    #[instrument(skip(self), fields(cycle_id = %cycle_id))]
    pub async fn update_billing_cycle_status(
        &self,
        cycle_id: Uuid,
        status: BillingCycleStatus,
        invoice_id: Option<Uuid>,
    ) -> Result<Option<BillingCycle>, AppError> {
        let cycle = sqlx::query_as::<_, BillingCycle>(
            r#"
            UPDATE billing_cycles
            SET status = $2, invoice_id = COALESCE($3, invoice_id)
            WHERE cycle_id = $1
            RETURNING cycle_id, subscription_id, period_start, period_end, status, invoice_id, created_utc, updated_utc
            "#,
        )
        .bind(cycle_id)
        .bind(status.as_str())
        .bind(invoice_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update billing cycle status: {}", e)))?;

        Ok(cycle)
    }

    /// Advance subscription to next billing cycle.
    #[instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn advance_billing_cycle(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<(BillingCycle, BillingCycle), AppError> {
        let subscription = self
            .get_subscription(tenant_id, subscription_id)
            .await?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Subscription not found")))?;

        if subscription.status != SubscriptionStatus::Active.as_str() {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Subscription must be active to advance billing cycle"
            )));
        }

        let plan = self
            .get_plan(tenant_id, subscription.plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Plan not found")))?;

        // Get current cycle
        let current_cycle = self
            .get_current_billing_cycle(subscription_id)
            .await?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("No current billing cycle")))?;

        // Calculate new period
        let interval = BillingInterval::from_string(&plan.billing_interval);
        let new_period_start = subscription.current_period_end;
        let new_period_end = calculate_period_end(new_period_start, interval, plan.interval_count);

        // Create new cycle
        let new_cycle = self
            .create_billing_cycle(subscription_id, new_period_start, new_period_end)
            .await?;

        // Update subscription periods
        sqlx::query(
            r#"
            UPDATE subscriptions
            SET current_period_start = $3, current_period_end = $4,
                plan_id = COALESCE(pending_plan_id, plan_id),
                pending_plan_id = NULL
            WHERE tenant_id = $1 AND subscription_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(new_period_start)
        .bind(new_period_end)
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to advance subscription: {}", e))
        })?;

        Ok((current_cycle, new_cycle))
    }

    // =========================================================================
    // Charge Operations
    // =========================================================================

    /// Create a charge.
    #[instrument(skip(self, input), fields(cycle_id = %input.cycle_id))]
    pub async fn create_charge(&self, input: &CreateCharge) -> Result<Charge, AppError> {
        let charge_id = Uuid::new_v4();
        let charge = sqlx::query_as::<_, Charge>(
            r#"
            INSERT INTO charges (charge_id, cycle_id, charge_type, description, quantity, unit_price, amount, is_prorated, proration_factor, component_id, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING charge_id, cycle_id, charge_type, description, quantity, unit_price, amount, is_prorated, proration_factor, component_id, metadata, created_utc
            "#,
        )
        .bind(charge_id)
        .bind(input.cycle_id)
        .bind(input.charge_type.as_str())
        .bind(&input.description)
        .bind(input.quantity)
        .bind(input.unit_price)
        .bind(input.amount)
        .bind(input.is_prorated)
        .bind(input.proration_factor)
        .bind(input.component_id)
        .bind(&input.metadata)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create charge: {}", e)))?;

        Ok(charge)
    }

    /// Get a charge by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, charge_id = %charge_id))]
    pub async fn get_charge(
        &self,
        tenant_id: Uuid,
        charge_id: Uuid,
    ) -> Result<Option<Charge>, AppError> {
        let charge = sqlx::query_as::<_, Charge>(
            r#"
            SELECT c.charge_id, c.cycle_id, c.charge_type, c.description, c.quantity, c.unit_price, c.amount, c.is_prorated, c.proration_factor, c.component_id, c.metadata, c.created_utc
            FROM charges c
            JOIN billing_cycles bc ON c.cycle_id = bc.cycle_id
            JOIN subscriptions s ON bc.subscription_id = s.subscription_id
            WHERE s.tenant_id = $1 AND c.charge_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(charge_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get charge: {}", e)))?;

        Ok(charge)
    }

    /// List charges for a billing cycle.
    #[instrument(skip(self, filter), fields(cycle_id = %cycle_id))]
    pub async fn list_charges(
        &self,
        tenant_id: Uuid,
        cycle_id: Uuid,
        filter: &ListChargesFilter,
    ) -> Result<Vec<Charge>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;
        let charge_type_str = filter.charge_type.map(|c| c.as_str().to_string());

        let charges = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, Charge>(
                r#"
                SELECT c.charge_id, c.cycle_id, c.charge_type, c.description, c.quantity, c.unit_price, c.amount, c.is_prorated, c.proration_factor, c.component_id, c.metadata, c.created_utc
                FROM charges c
                JOIN billing_cycles bc ON c.cycle_id = bc.cycle_id
                JOIN subscriptions s ON bc.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND c.cycle_id = $2
                  AND ($3::varchar IS NULL OR c.charge_type = $3)
                  AND c.charge_id > $4
                ORDER BY c.charge_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(cycle_id)
            .bind(&charge_type_str)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, Charge>(
                r#"
                SELECT c.charge_id, c.cycle_id, c.charge_type, c.description, c.quantity, c.unit_price, c.amount, c.is_prorated, c.proration_factor, c.component_id, c.metadata, c.created_utc
                FROM charges c
                JOIN billing_cycles bc ON c.cycle_id = bc.cycle_id
                JOIN subscriptions s ON bc.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND c.cycle_id = $2
                  AND ($3::varchar IS NULL OR c.charge_type = $3)
                ORDER BY c.charge_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(cycle_id)
            .bind(&charge_type_str)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list charges: {}", e)))?;

        Ok(charges)
    }
}
