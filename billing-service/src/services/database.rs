//! Database service for billing-service.

use crate::models::{
    BillingCycle, BillingCycleStatus, BillingInterval, BillingPlan, BillingRun, BillingRunResult,
    BillingRunStatus, BillingRunType, Charge, CreateCharge, CreatePlan, CreateSubscription,
    CreateUsageComponent, ListBillingCyclesFilter, ListBillingRunsFilter, ListChargesFilter,
    ListPlansFilter, ListSubscriptionsFilter, ListUsageFilter, ProrationMode, RecordUsage,
    Subscription, SubscriptionStatus, UpdatePlan, UsageComponent, UsageComponentSummary,
    UsageRecord,
};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use service_core::error::AppError;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool.
    #[instrument(skip(database_url), fields(service = "billing-service"))]
    pub async fn new(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self, AppError> {
        info!(
            max_connections = max_connections,
            min_connections = min_connections,
            "Connecting to PostgreSQL"
        );

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to connect: {}", e)))?;

        info!("PostgreSQL connection pool established");

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check database health.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["health_check"])
            .start_timer();

        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Health check failed: {}", e)))?;

        timer.observe_duration();
        Ok(())
    }

    /// Run database migrations.
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<(), AppError> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Migration failed: {}", e)))?;
        info!("Database migrations completed");
        Ok(())
    }

    // =========================================================================
    // Plan Operations
    // =========================================================================

    /// Create a new billing plan.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_plan(&self, input: &CreatePlan) -> Result<BillingPlan, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_plan"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create plan: {}", e)))?;

        timer.observe_duration();
        info!(plan_id = %plan.plan_id, name = %plan.name, "Plan created");

        Ok(plan)
    }

    /// Create a usage component for a plan.
    #[instrument(skip(self, input), fields(plan_id = %input.plan_id))]
    pub async fn create_usage_component(
        &self,
        input: &CreateUsageComponent,
    ) -> Result<UsageComponent, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_usage_component"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create usage component: {}", e)))?;

        timer.observe_duration();

        Ok(component)
    }

    /// Get a plan by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, plan_id = %plan_id))]
    pub async fn get_plan(
        &self,
        tenant_id: Uuid,
        plan_id: Uuid,
    ) -> Result<Option<BillingPlan>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_plan"])
            .start_timer();

        let plan = sqlx::query_as::<_, BillingPlan>(
            r#"
            SELECT plan_id, tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, is_active, is_archived, metadata, created_utc, updated_utc
            FROM billing_plans
            WHERE tenant_id = $1 AND plan_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get plan: {}", e)))?;

        timer.observe_duration();

        Ok(plan)
    }

    /// Get usage components for a plan.
    #[instrument(skip(self), fields(plan_id = %plan_id))]
    pub async fn get_usage_components(
        &self,
        plan_id: Uuid,
    ) -> Result<Vec<UsageComponent>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_usage_components"])
            .start_timer();

        let components = sqlx::query_as::<_, UsageComponent>(
            r#"
            SELECT component_id, plan_id, name, unit_name, unit_price, included_units, is_active, created_utc
            FROM usage_components
            WHERE plan_id = $1 AND is_active = TRUE
            ORDER BY name
            "#,
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get usage components: {}", e)))?;

        timer.observe_duration();

        Ok(components)
    }

    /// List plans for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_plans(
        &self,
        tenant_id: Uuid,
        filter: &ListPlansFilter,
    ) -> Result<Vec<BillingPlan>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_plans"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list plans: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_plan"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update plan: {}", e)))?;

        timer.observe_duration();

        Ok(plan)
    }

    /// Archive a plan.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, plan_id = %plan_id))]
    pub async fn archive_plan(
        &self,
        tenant_id: Uuid,
        plan_id: Uuid,
    ) -> Result<Option<BillingPlan>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["archive_plan"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to archive plan: {}", e)))?;

        timer.observe_duration();

        if let Some(ref p) = plan {
            info!(plan_id = %p.plan_id, "Plan archived");
        }

        Ok(plan)
    }

    // =========================================================================
    // Subscription Operations
    // =========================================================================

    /// Create a new subscription.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_subscription(
        &self,
        input: &CreateSubscription,
    ) -> Result<Subscription, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_subscription"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create subscription: {}", e)))?;

        timer.observe_duration();
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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_subscription"])
            .start_timer();

        let subscription = sqlx::query_as::<_, Subscription>(
            r#"
            SELECT subscription_id, tenant_id, customer_id, plan_id, status, billing_anchor_day, start_date, end_date, trial_end_date, current_period_start, current_period_end, proration_mode, pending_plan_id, metadata, created_utc, updated_utc
            FROM subscriptions
            WHERE tenant_id = $1 AND subscription_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get subscription: {}", e)))?;

        timer.observe_duration();

        Ok(subscription)
    }

    /// List subscriptions for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_subscriptions(
        &self,
        tenant_id: Uuid,
        filter: &ListSubscriptionsFilter,
    ) -> Result<Vec<Subscription>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_subscriptions"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list subscriptions: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_subscription_status"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update subscription status: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["change_subscription_plan"])
            .start_timer();

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
                .fetch_optional(&self.pool)
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
                .fetch_optional(&self.pool)
                .await
            }
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to change plan: {}", e)))?;

        timer.observe_duration();

        Ok(subscription)
    }

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_billing_cycle"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing cycle: {}", e)))?;

        timer.observe_duration();

        Ok(cycle)
    }

    /// Get a billing cycle by ID with tenant isolation via subscription.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, cycle_id = %cycle_id))]
    pub async fn get_billing_cycle(
        &self,
        tenant_id: Uuid,
        cycle_id: Uuid,
    ) -> Result<Option<BillingCycle>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_billing_cycle"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing cycle: {}", e)))?;

        timer.observe_duration();

        Ok(cycle)
    }

    /// Get the current (pending) billing cycle for a subscription.
    #[instrument(skip(self), fields(subscription_id = %subscription_id))]
    pub async fn get_current_billing_cycle(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<BillingCycle>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_current_billing_cycle"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get current billing cycle: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_billing_cycles"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list billing cycles: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_billing_cycle_status"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update billing cycle status: {}", e)))?;

        timer.observe_duration();

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
        .execute(&self.pool)
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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_charge"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create charge: {}", e)))?;

        timer.observe_duration();

        Ok(charge)
    }

    /// Get a charge by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, charge_id = %charge_id))]
    pub async fn get_charge(
        &self,
        tenant_id: Uuid,
        charge_id: Uuid,
    ) -> Result<Option<Charge>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_charge"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get charge: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_charges"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list charges: {}", e)))?;

        timer.observe_duration();

        Ok(charges)
    }

    // =========================================================================
    // Usage Operations
    // =========================================================================

    /// Record usage with idempotency.
    #[instrument(skip(self, input), fields(subscription_id = %input.subscription_id))]
    pub async fn record_usage(&self, input: &RecordUsage) -> Result<UsageRecord, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["record_usage"])
            .start_timer();

        // Check for existing record with same idempotency key
        let existing = sqlx::query_as::<_, UsageRecord>(
            r#"
            SELECT record_id, subscription_id, component_id, idempotency_key, quantity, timestamp, cycle_id, is_invoiced, metadata, created_utc
            FROM usage_records
            WHERE idempotency_key = $1
            "#,
        )
        .bind(&input.idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to check idempotency: {}", e)))?;

        if let Some(record) = existing {
            timer.observe_duration();
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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                // Race condition: another request created the record
                AppError::Conflict(anyhow::anyhow!("Duplicate idempotency key"))
            }
            _ => AppError::DatabaseError(anyhow::anyhow!("Failed to record usage: {}", e)),
        })?;

        timer.observe_duration();

        Ok(record)
    }

    /// Get a usage record by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, record_id = %record_id))]
    pub async fn get_usage_record(
        &self,
        tenant_id: Uuid,
        record_id: Uuid,
    ) -> Result<Option<UsageRecord>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_usage_record"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get usage record: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_usage_records"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list usage records: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_usage_summary"])
            .start_timer();

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
            .fetch_one(&self.pool)
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

        timer.observe_duration();

        Ok(summaries)
    }

    /// Mark usage records as invoiced.
    #[instrument(skip(self), fields(cycle_id = %cycle_id))]
    pub async fn mark_usage_invoiced(&self, cycle_id: Uuid) -> Result<u64, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["mark_usage_invoiced"])
            .start_timer();

        let result = sqlx::query(
            r#"
            UPDATE usage_records
            SET is_invoiced = TRUE
            WHERE cycle_id = $1 AND is_invoiced = FALSE
            "#,
        )
        .bind(cycle_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to mark usage invoiced: {}", e))
        })?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_billing_run"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing run: {}", e)))?;

        timer.observe_duration();

        Ok(run)
    }

    /// Get a billing run by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, run_id = %run_id))]
    pub async fn get_billing_run(
        &self,
        tenant_id: Uuid,
        run_id: Uuid,
    ) -> Result<Option<BillingRun>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_billing_run"])
            .start_timer();

        let run = sqlx::query_as::<_, BillingRun>(
            r#"
            SELECT run_id, tenant_id, run_type, status, started_utc, completed_utc, subscriptions_processed, subscriptions_succeeded, subscriptions_failed, error_message
            FROM billing_runs
            WHERE tenant_id = $1 AND run_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing run: {}", e)))?;

        timer.observe_duration();

        Ok(run)
    }

    /// List billing runs for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_billing_runs(
        &self,
        tenant_id: Uuid,
        filter: &ListBillingRunsFilter,
    ) -> Result<Vec<BillingRun>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_billing_runs"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list billing runs: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_billing_run"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update billing run: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_billing_run_result"])
            .start_timer();

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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create billing run result: {}", e)))?;

        timer.observe_duration();

        Ok(result)
    }

    /// Get billing run results.
    #[instrument(skip(self), fields(run_id = %run_id))]
    pub async fn get_billing_run_results(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<BillingRunResult>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_billing_run_results"])
            .start_timer();

        let results = sqlx::query_as::<_, BillingRunResult>(
            r#"
            SELECT result_id, run_id, subscription_id, status, invoice_id, error_message, created_utc
            FROM billing_run_results
            WHERE run_id = $1
            ORDER BY created_utc
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get billing run results: {}", e)))?;

        timer.observe_duration();

        Ok(results)
    }

    /// Find subscriptions due for billing.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn find_subscriptions_due_for_billing(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<Subscription>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["find_subscriptions_due_for_billing"])
            .start_timer();

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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to find subscriptions due for billing: {}", e)))?;

        timer.observe_duration();

        Ok(subscriptions)
    }
}

/// Calculate the end date for a billing period.
fn calculate_period_end(start: NaiveDate, interval: BillingInterval, count: i32) -> NaiveDate {
    use chrono::Months;

    match interval {
        BillingInterval::Daily => start + chrono::Duration::days(count as i64),
        BillingInterval::Weekly => start + chrono::Duration::weeks(count as i64),
        BillingInterval::Monthly => start + Months::new(count as u32),
        BillingInterval::Quarterly => start + Months::new((count * 3) as u32),
        BillingInterval::Annually => start + Months::new((count * 12) as u32),
    }
}
