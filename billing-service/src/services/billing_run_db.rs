//! Billing run and usage recording database operations.
#![allow(clippy::too_many_arguments)]

use crate::models::{
    BillingRun, BillingRunResult, BillingRunStatus, BillingRunType, ListBillingRunsFilter,
    ListUsageFilter, RecordUsage, Subscription, UsageComponentSummary, UsageRecord,
};
use crate::services::database::Database;
use chrono::Utc;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::instrument;
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Usage Operations
    // =========================================================================

    /// Record usage with idempotency.
    #[instrument(skip(self, input), fields(subscription_id = %input.subscription_id))]
    pub async fn record_usage(&self, input: &RecordUsage) -> Result<UsageRecord, AppError> {
        // Check for existing record with same idempotency key
        let existing = sqlx::query_as::<_, UsageRecord>(
            r#"
            SELECT record_id, subscription_id, component_id, idempotency_key, quantity, timestamp, cycle_id, is_invoiced, metadata, created_utc
            FROM usage_records
            WHERE idempotency_key = $1
            "#,
        )
        .bind(&input.idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to check idempotency: {}", e)))?;

        if let Some(record) = existing {
            return Ok(record);
        }

        // Get current billing cycle
        let cycle = self
            .get_current_billing_cycle(input.subscription_id)
            .await?;

        let record_id = Uuid::new_v4();
        let record = sqlx::query_as::<_, UsageRecord>(
            r#"
            INSERT INTO usage_records (record_id, subscription_id, component_id, idempotency_key, quantity, timestamp, cycle_id, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING record_id, subscription_id, component_id, idempotency_key, quantity, timestamp, cycle_id, is_invoiced, metadata, created_utc
            "#,
        )
        .bind(record_id)
        .bind(input.subscription_id)
        .bind(input.component_id)
        .bind(&input.idempotency_key)
        .bind(input.quantity)
        .bind(input.timestamp)
        .bind(cycle.map(|c| c.cycle_id))
        .bind(&input.metadata)
        .fetch_one(self.pool())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                // Race condition: another request created the record
                AppError::Conflict(anyhow::anyhow!("Duplicate idempotency key"))
            }
            _ => AppError::DatabaseError(anyhow::anyhow!("Failed to record usage: {}", e)),
        })?;

        Ok(record)
    }

    /// Get a usage record by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, record_id = %record_id))]
    pub async fn get_usage_record(
        &self,
        tenant_id: Uuid,
        record_id: Uuid,
    ) -> Result<Option<UsageRecord>, AppError> {
        let record = sqlx::query_as::<_, UsageRecord>(
            r#"
            SELECT ur.record_id, ur.subscription_id, ur.component_id, ur.idempotency_key, ur.quantity, ur.timestamp, ur.cycle_id, ur.is_invoiced, ur.metadata, ur.created_utc
            FROM usage_records ur
            JOIN subscriptions s ON ur.subscription_id = s.subscription_id
            WHERE s.tenant_id = $1 AND ur.record_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(record_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get usage record: {}", e)))?;

        Ok(record)
    }

    /// List usage records for a subscription.
    #[instrument(skip(self, filter), fields(subscription_id = %subscription_id))]
    pub async fn list_usage_records(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
        filter: &ListUsageFilter,
    ) -> Result<Vec<UsageRecord>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;

        let records = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, UsageRecord>(
                r#"
                SELECT ur.record_id, ur.subscription_id, ur.component_id, ur.idempotency_key, ur.quantity, ur.timestamp, ur.cycle_id, ur.is_invoiced, ur.metadata, ur.created_utc
                FROM usage_records ur
                JOIN subscriptions s ON ur.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND ur.subscription_id = $2
                  AND ($3::uuid IS NULL OR ur.component_id = $3)
                  AND ($4::uuid IS NULL OR ur.cycle_id = $4)
                  AND ($5::bool IS NULL OR ur.is_invoiced = $5)
                  AND ur.record_id > $6
                ORDER BY ur.record_id
                LIMIT $7
                "#,
            )
            .bind(tenant_id)
            .bind(subscription_id)
            .bind(filter.component_id)
            .bind(filter.cycle_id)
            .bind(filter.is_invoiced)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, UsageRecord>(
                r#"
                SELECT ur.record_id, ur.subscription_id, ur.component_id, ur.idempotency_key, ur.quantity, ur.timestamp, ur.cycle_id, ur.is_invoiced, ur.metadata, ur.created_utc
                FROM usage_records ur
                JOIN subscriptions s ON ur.subscription_id = s.subscription_id
                WHERE s.tenant_id = $1 AND ur.subscription_id = $2
                  AND ($3::uuid IS NULL OR ur.component_id = $3)
                  AND ($4::uuid IS NULL OR ur.cycle_id = $4)
                  AND ($5::bool IS NULL OR ur.is_invoiced = $5)
                ORDER BY ur.record_id
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(subscription_id)
            .bind(filter.component_id)
            .bind(filter.cycle_id)
            .bind(filter.is_invoiced)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list usage records: {}", e)))?;

        Ok(records)
    }

    /// Get usage summary for a subscription and billing cycle.
    #[instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn get_usage_summary(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
        cycle_id: Option<Uuid>,
    ) -> Result<Vec<UsageComponentSummary>, AppError> {
        // Get actual cycle_id
        let actual_cycle_id = if let Some(id) = cycle_id {
            id
        } else {
            self.get_current_billing_cycle(subscription_id)
                .await?
                .map(|c| c.cycle_id)
                .unwrap_or(Uuid::nil())
        };

        // Get subscription's plan
        let subscription = self
            .get_subscription(tenant_id, subscription_id)
            .await?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Subscription not found")))?;

        let components = self.get_usage_components(subscription.plan_id).await?;

        let mut summaries = Vec::new();

        for component in components {
            // Aggregate usage for this component in the cycle
            let total: Option<Decimal> = sqlx::query_scalar(
                r#"
                SELECT COALESCE(SUM(quantity), 0)
                FROM usage_records
                WHERE subscription_id = $1 AND component_id = $2 AND cycle_id = $3
                "#,
            )
            .bind(subscription_id)
            .bind(component.component_id)
            .bind(actual_cycle_id)
            .fetch_one(self.pool())
            .await
            .map_err(|e| {
                AppError::DatabaseError(anyhow::anyhow!("Failed to aggregate usage: {}", e))
            })?;

            let total_quantity = total.unwrap_or(Decimal::ZERO);
            let included = Decimal::from(component.included_units);
            let billable_units = (total_quantity - included).max(Decimal::ZERO);
            let amount = billable_units * component.unit_price;

            summaries.push(UsageComponentSummary {
                component_id: component.component_id,
                name: component.name,
                total_quantity,
                included_units: component.included_units,
                billable_units,
                amount,
            });
        }

        Ok(summaries)
    }

    /// Mark usage records as invoiced.
    #[instrument(skip(self), fields(cycle_id = %cycle_id))]
    pub async fn mark_usage_invoiced(&self, cycle_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE usage_records
            SET is_invoiced = TRUE
            WHERE cycle_id = $1 AND is_invoiced = FALSE
            "#,
        )
        .bind(cycle_id)
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to mark usage invoiced: {}", e))
        })?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // Billing Run Operations
    // =========================================================================

    /// Create a billing run.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn create_billing_run(
        &self,
        tenant_id: Uuid,
        run_type: BillingRunType,
    ) -> Result<BillingRun, AppError> {
        let run_id = Uuid::new_v4();
        let run = sqlx::query_as::<_, BillingRun>(
            r#"
            INSERT INTO billing_runs (run_id, tenant_id, run_type)
            VALUES ($1, $2, $3)
            RETURNING run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
            "#,
        )
        .bind(run_id)
        .bind(tenant_id)
        .bind(run_type.as_str())
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing run: {}", e)))?;

        Ok(run)
    }

    /// Get a billing run by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, run_id = %run_id))]
    pub async fn get_billing_run(
        &self,
        tenant_id: Uuid,
        run_id: Uuid,
    ) -> Result<Option<BillingRun>, AppError> {
        let run = sqlx::query_as::<_, BillingRun>(
            r#"
            SELECT run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
            FROM billing_runs
            WHERE tenant_id = $1 AND run_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(run_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing run: {}", e)))?;

        Ok(run)
    }

    /// List billing runs for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_billing_runs(
        &self,
        tenant_id: Uuid,
        filter: &ListBillingRunsFilter,
    ) -> Result<Vec<BillingRun>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;
        let status_str = filter.status.map(|s| s.as_str().to_string());
        let run_type_str = filter.run_type.map(|r| r.as_str().to_string());

        let runs = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, BillingRun>(
                r#"
                SELECT run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
                FROM billing_runs
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR status = $2)
                  AND ($3::varchar IS NULL OR run_type = $3)
                  AND run_id > $4
                ORDER BY run_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(&status_str)
            .bind(&run_type_str)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BillingRun>(
                r#"
                SELECT run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
                FROM billing_runs
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR status = $2)
                  AND ($3::varchar IS NULL OR run_type = $3)
                ORDER BY run_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(&status_str)
            .bind(&run_type_str)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list billing runs: {}", e)))?;

        Ok(runs)
    }

    /// Update billing run status and counts.
    #[instrument(skip(self), fields(run_id = %run_id))]
    pub async fn update_billing_run(
        &self,
        run_id: Uuid,
        status: BillingRunStatus,
        processed: i32,
        succeeded: i32,
        failed: i32,
        error_message: Option<String>,
    ) -> Result<Option<BillingRun>, AppError> {
        let completed_utc = if status != BillingRunStatus::Running {
            Some(Utc::now())
        } else {
            None
        };

        let run = sqlx::query_as::<_, BillingRun>(
            r#"
            UPDATE billing_runs
            SET status = $2, completed_utc = COALESCE($3, completed_utc), subscriptions_processed = $4, subscriptions_succeeded = $5, subscriptions_failed = $6, error_message = $7
            WHERE run_id = $1
            RETURNING run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
            "#,
        )
        .bind(run_id)
        .bind(status.as_str())
        .bind(completed_utc)
        .bind(processed)
        .bind(succeeded)
        .bind(failed)
        .bind(error_message)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update billing run: {}", e)))?;

        Ok(run)
    }

    /// Record a billing run result.
    #[instrument(skip(self), fields(run_id = %run_id, subscription_id = %subscription_id))]
    pub async fn create_billing_run_result(
        &self,
        run_id: Uuid,
        subscription_id: Uuid,
        status: &str,
        invoice_id: Option<Uuid>,
        error_message: Option<String>,
    ) -> Result<BillingRunResult, AppError> {
        let result_id = Uuid::new_v4();
        let result = sqlx::query_as::<_, BillingRunResult>(
            r#"
            INSERT INTO billing_run_results (result_id, run_id, subscription_id, status, invoice_id, error_message)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING result_id, run_id, subscription_id, status, invoice_id, error_message, created_utc
            "#,
        )
        .bind(result_id)
        .bind(run_id)
        .bind(subscription_id)
        .bind(status)
        .bind(invoice_id)
        .bind(error_message)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing run result: {}", e)))?;

        Ok(result)
    }

    /// Get billing run results.
    #[instrument(skip(self), fields(run_id = %run_id))]
    pub async fn get_billing_run_results(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<BillingRunResult>, AppError> {
        let results = sqlx::query_as::<_, BillingRunResult>(
            r#"
            SELECT result_id, run_id, subscription_id, status, invoice_id, error_message, created_utc
            FROM billing_run_results
            WHERE run_id = $1
            ORDER BY created_utc
            "#,
        )
        .bind(run_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing run results: {}", e)))?;

        Ok(results)
    }

    /// Find subscriptions due for billing.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn find_subscriptions_due_for_billing(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<Subscription>, AppError> {
        let today = Utc::now().date_naive();

        let subscriptions = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
            FROM subscriptions
            WHERE tenant_id = $1
              AND status = 'active'
              AND current_period_end <= $2
            "#,
        )
        .bind(tenant_id)
        .bind(today)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to find subscriptions due for billing: {}", e)))?;

        Ok(subscriptions)
    }
}
