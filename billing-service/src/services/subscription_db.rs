//! Subscription database operations.
#![allow(clippy::too_many_arguments)]

use crate::models::{
    BillingInterval, CreateSubscription, ListSubscriptionsFilter, ProrationMode, Subscription,
    SubscriptionStatus,
};
use crate::services::database::Database;
use chrono::NaiveDate;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Subscription Operations
    // =========================================================================

    /// Create a new subscription.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_subscription(
        &self,
        input: &CreateSubscription,
    ) -> Result<Subscription, AppError> {
        let subscription_id = Uuid::new_v4();
        let status = if input.trial_end_date.is_some() {
            SubscriptionStatus::Trial
        } else {
            SubscriptionStatus::Active
        };

        // Calculate initial period based on plan
        let plan = self
            .get_plan(input.tenant_id, input.plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Plan not found")))?;

        let interval = BillingInterval::from_string(&plan.billing_interval);
        let period_end = calculate_period_end(input.start_date, interval, plan.interval_count);

        let subscription = sqlx::query_as::<_, Subscription>(
            r#"
            INSERT INTO subscriptions (subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, trial_end_date, current_period_start, current_period_end, proration_mode, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
            "#,
        )
        .bind(subscription_id)
        .bind(input.tenant_id)
        .bind(input.customer_id)
        .bind(input.plan_id)
        .bind(status.as_str())
        .bind(input.billing_anchor_day)
        .bind(input.start_date)
        .bind(input.trial_end_date)
        .bind(input.start_date)
        .bind(period_end)
        .bind(input.proration_mode.as_str())
        .bind(&input.metadata)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create subscription: {}", e)))?;

        info!(subscription_id = %subscription.subscription_id, "Subscription created");

        Ok(subscription)
    }

    /// Get a subscription by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, subscription_id = %subscription_id))]
    pub async fn get_subscription(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<Option<Subscription>, AppError> {
        let subscription = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
            FROM subscriptions
            WHERE tenant_id = $1 AND subscription_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get subscription: {}", e)))?;

        Ok(subscription)
    }

    /// List subscriptions for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_subscriptions(
        &self,
        tenant_id: Uuid,
        filter: &ListSubscriptionsFilter,
    ) -> Result<Vec<Subscription>, AppError> {
        let limit = filter.page_size.clamp(1, 100) as i64;
        let status_str = filter.status.map(|s| s.as_str().to_string());

        let subscriptions = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, Subscription>(
                r#"
                SELECT subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
                FROM subscriptions
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR customer_id = $2)
                  AND ($3::varchar IS NULL OR status = $3)
                  AND ($4::uuid IS NULL OR plan_id = $4)
                  AND subscription_id > $5
                ORDER BY subscription_id
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(filter.customer_id)
            .bind(&status_str)
            .bind(filter.plan_id)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, Subscription>(
                r#"
                SELECT subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
                FROM subscriptions
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR customer_id = $2)
                  AND ($3::varchar IS NULL OR status = $3)
                  AND ($4::uuid IS NULL OR plan_id = $4)
                ORDER BY subscription_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(filter.customer_id)
            .bind(&status_str)
            .bind(filter.plan_id)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list subscriptions: {}", e)))?;

        Ok(subscriptions)
    }

    /// Update subscription status.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, subscription_id = %subscription_id))]
    pub async fn update_subscription_status(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
        status: SubscriptionStatus,
        end_date: Option<NaiveDate>,
    ) -> Result<Option<Subscription>, AppError> {
        let subscription = sqlx::query_as::<_, Subscription>(
            r#"
            UPDATE subscriptions
            SET status = $3, end_date = COALESCE($4, end_date), trial_end_date = CASE WHEN $3 = 'active' THEN NULL ELSE trial_end_date END
            WHERE tenant_id = $1 AND subscription_id = $2
            RETURNING subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(status.as_str())
        .bind(end_date)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update subscription status: {}", e)))?;

        Ok(subscription)
    }

    /// Change subscription plan.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, subscription_id = %subscription_id))]
    pub async fn change_subscription_plan(
        &self,
        tenant_id: Uuid,
        subscription_id: Uuid,
        new_plan_id: Uuid,
        mode: ProrationMode,
    ) -> Result<Option<Subscription>, AppError> {
        let subscription = match mode {
            ProrationMode::Immediate | ProrationMode::None => {
                // Change plan immediately
                sqlx::query_as::<_, Subscription>(
                    r#"
                    UPDATE subscriptions
                    SET plan_id = $3, pending_plan_id = NULL
                    WHERE tenant_id = $1 AND subscription_id = $2 AND status = 'active'
                    RETURNING subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
                    "#,
                )
                .bind(tenant_id)
                .bind(subscription_id)
                .bind(new_plan_id)
                .fetch_optional(self.pool())
                .await
            }
            ProrationMode::NextCycle => {
                // Schedule plan change for next cycle
                sqlx::query_as::<_, Subscription>(
                    r#"
                    UPDATE subscriptions
                    SET pending_plan_id = $3
                    WHERE tenant_id = $1 AND subscription_id = $2 AND status = 'active'
                    RETURNING subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
                    "#,
                )
                .bind(tenant_id)
                .bind(subscription_id)
                .bind(new_plan_id)
                .fetch_optional(self.pool())
                .await
            }
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to change plan: {}", e)))?;

        Ok(subscription)
    }
}

/// Calculate the end date for a billing period.
pub(crate) fn calculate_period_end(
    start: NaiveDate,
    interval: BillingInterval,
    count: i32,
) -> NaiveDate {
    use chrono::Months;

    match interval {
        BillingInterval::Daily => start + chrono::Duration::days(count as i64),
        BillingInterval::Weekly => start + chrono::Duration::weeks(count as i64),
        BillingInterval::Monthly => start + Months::new(count as u32),
        BillingInterval::Quarterly => start + Months::new((count * 3) as u32),
        BillingInterval::Annually => start + Months::new((count * 12) as u32),
    }
}
