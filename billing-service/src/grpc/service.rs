//! BillingService gRPC implementation.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::billing_service_server::BillingService;
use crate::grpc::proto::*;
use crate::models::{
    BillingCycleStatus, BillingInterval, BillingRunStatus, BillingRunType, ChargeType,
    CreateCharge, CreatePlan, CreateSubscription, CreateUsageComponent, ListBillingCyclesFilter,
    ListBillingRunsFilter, ListChargesFilter, ListPlansFilter, ListSubscriptionsFilter,
    ListUsageFilter, ProrationMode, RecordUsage, SubscriptionStatus, UpdatePlan,
};
use crate::services::{
    record_billing_run, record_charge_amount, record_charge_created, record_error,
    record_grpc_request, record_grpc_request_duration, record_plan_operation,
    record_subscription_operation, record_usage_operation, Database,
};
use chrono::{Datelike, NaiveDate, Utc};
use prost_types::Timestamp;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// BillingService implementation.
pub struct BillingServiceImpl {
    db: Arc<Database>,
    capability_checker: Arc<CapabilityChecker>,
}

impl BillingServiceImpl {
    /// Create a new BillingServiceImpl.
    pub fn new(db: Arc<Database>, capability_checker: Arc<CapabilityChecker>) -> Self {
        Self {
            db,
            capability_checker,
        }
    }
}

// Helper functions for type conversions
#[allow(clippy::result_large_err)]
fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid UUID: {}", s)))
}

#[allow(clippy::result_large_err)]
fn parse_decimal(s: &str) -> Result<Decimal, Status> {
    Decimal::from_str(s).map_err(|_| Status::invalid_argument(format!("Invalid decimal: {}", s)))
}

#[allow(clippy::result_large_err)]
fn parse_date(s: &str) -> Result<NaiveDate, Status> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| Status::invalid_argument(format!("Invalid date: {}", s)))
}

#[allow(clippy::result_large_err)]
fn parse_tenant_id(auth: &crate::grpc::capability_check::AuthContext) -> Result<Uuid, Status> {
    Uuid::parse_str(&auth.tenant_id)
        .map_err(|_| Status::internal(format!("Invalid tenant_id: {}", auth.tenant_id)))
}

fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

fn timestamp_to_datetime(ts: Option<Timestamp>) -> chrono::DateTime<Utc> {
    ts.map(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32).unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now)
}

fn plan_to_proto(
    plan: crate::models::BillingPlan,
    components: Vec<crate::models::UsageComponent>,
) -> BillingPlan {
    BillingPlan {
        plan_id: plan.plan_id.to_string(),
        tenant_id: plan.tenant_id.to_string(),
        name: plan.name,
        description: plan.description.unwrap_or_default(),
        billing_interval: BillingInterval::from_string(&plan.billing_interval).to_proto(),
        interval_count: plan.interval_count,
        base_price: plan.base_price.to_string(),
        currency: plan.currency,
        tax_rate_id: plan
            .tax_rate_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        is_active: plan.is_active,
        is_archived: plan.is_archived,
        usage_components: components.into_iter().map(component_to_proto).collect(),
        metadata: plan.metadata.map(|m| m.to_string()).unwrap_or_default(),
        created_at: datetime_to_timestamp(plan.created_utc),
        updated_at: datetime_to_timestamp(plan.updated_utc),
    }
}

fn component_to_proto(c: crate::models::UsageComponent) -> UsageComponent {
    UsageComponent {
        component_id: c.component_id.to_string(),
        plan_id: c.plan_id.to_string(),
        name: c.name,
        unit_name: c.unit_name,
        unit_price: c.unit_price.to_string(),
        included_units: c.included_units,
        is_active: c.is_active,
    }
}

fn subscription_to_proto(s: crate::models::Subscription) -> Subscription {
    Subscription {
        subscription_id: s.subscription_id.to_string(),
        tenant_id: s.tenant_id.to_string(),
        customer_id: s.customer_id.to_string(),
        plan_id: s.plan_id.to_string(),
        status: SubscriptionStatus::from_string(&s.status).to_proto(),
        billing_anchor_day: s.billing_anchor_day,
        start_date: s.start_date.to_string(),
        end_date: s.end_date.map(|d| d.to_string()).unwrap_or_default(),
        trial_end_date: s.trial_end_date.map(|d| d.to_string()).unwrap_or_default(),
        current_period_start: s.current_period_start.to_string(),
        current_period_end: s.current_period_end.to_string(),
        proration_mode: ProrationMode::from_string(&s.proration_mode).to_proto(),
        pending_plan_id: s
            .pending_plan_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        metadata: s.metadata.map(|m| m.to_string()).unwrap_or_default(),
        created_at: datetime_to_timestamp(s.created_utc),
        updated_at: datetime_to_timestamp(s.updated_utc),
    }
}

fn cycle_to_proto(c: crate::models::BillingCycle, charges: Vec<Charge>) -> BillingCycle {
    BillingCycle {
        cycle_id: c.cycle_id.to_string(),
        subscription_id: c.subscription_id.to_string(),
        period_start: c.period_start.to_string(),
        period_end: c.period_end.to_string(),
        status: BillingCycleStatus::from_string(&c.status).to_proto(),
        invoice_id: c.invoice_id.map(|id| id.to_string()).unwrap_or_default(),
        charges,
        created_at: datetime_to_timestamp(c.created_utc),
        updated_at: datetime_to_timestamp(c.updated_utc),
    }
}

fn charge_to_proto(c: crate::models::Charge) -> Charge {
    Charge {
        charge_id: c.charge_id.to_string(),
        cycle_id: c.cycle_id.to_string(),
        charge_type: ChargeType::from_string(&c.charge_type).to_proto(),
        description: c.description,
        quantity: c.quantity.to_string(),
        unit_price: c.unit_price.to_string(),
        amount: c.amount.to_string(),
        is_prorated: c.is_prorated,
        proration_factor: c
            .proration_factor
            .map(|f| f.to_string())
            .unwrap_or_default(),
        component_id: c.component_id.map(|id| id.to_string()).unwrap_or_default(),
        metadata: c.metadata.map(|m| m.to_string()).unwrap_or_default(),
        created_at: datetime_to_timestamp(c.created_utc),
    }
}

fn usage_record_to_proto(r: crate::models::UsageRecord) -> UsageRecord {
    UsageRecord {
        record_id: r.record_id.to_string(),
        subscription_id: r.subscription_id.to_string(),
        component_id: r.component_id.to_string(),
        idempotency_key: r.idempotency_key,
        quantity: r.quantity.to_string(),
        timestamp: datetime_to_timestamp(r.timestamp),
        cycle_id: r.cycle_id.map(|id| id.to_string()).unwrap_or_default(),
        is_invoiced: r.is_invoiced,
        metadata: r.metadata.map(|m| m.to_string()).unwrap_or_default(),
        created_at: datetime_to_timestamp(r.created_utc),
    }
}

fn billing_run_to_proto(
    r: crate::models::BillingRun,
    results: Vec<BillingRunResult>,
) -> BillingRun {
    BillingRun {
        run_id: r.run_id.to_string(),
        tenant_id: r.tenant_id.to_string(),
        run_type: BillingRunType::from_string(&r.run_type).to_proto(),
        status: BillingRunStatus::from_string(&r.status).to_proto(),
        started_at: datetime_to_timestamp(r.started_utc),
        completed_at: r.completed_utc.and_then(datetime_to_timestamp),
        subscriptions_processed: r.subscriptions_processed,
        subscriptions_succeeded: r.subscriptions_succeeded,
        subscriptions_failed: r.subscriptions_failed,
        error_message: r.error_message.unwrap_or_default(),
        results,
    }
}

fn billing_run_result_to_proto(r: crate::models::BillingRunResult) -> BillingRunResult {
    BillingRunResult {
        result_id: r.result_id.to_string(),
        run_id: r.run_id.to_string(),
        subscription_id: r.subscription_id.to_string(),
        status: r.status,
        invoice_id: r.invoice_id.map(|id| id.to_string()).unwrap_or_default(),
        error_message: r.error_message.unwrap_or_default(),
        created_at: datetime_to_timestamp(r.created_utc),
    }
}

#[tonic::async_trait]
impl BillingService for BillingServiceImpl {
    // =========================================================================
    // Plan Management
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "CreatePlan"))]
    async fn create_plan(
        &self,
        request: Request<CreatePlanRequest>,
    ) -> Result<Response<CreatePlanResponse>, Status> {
        let start = Instant::now();
        let method = "CreatePlan";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_PLAN_CREATE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        tracing::info!(tenant_id = %tenant_id, name = %req.name, "Creating plan");

        let input = CreatePlan {
            tenant_id,
            name: req.name,
            description: if req.description.is_empty() {
                None
            } else {
                Some(req.description)
            },
            billing_interval: BillingInterval::from_proto(req.billing_interval),
            interval_count: if req.interval_count > 0 {
                req.interval_count
            } else {
                1
            },
            base_price: parse_decimal(&req.base_price)?,
            currency: if req.currency.is_empty() {
                "USD".to_string()
            } else {
                req.currency
            },
            tax_rate_id: if req.tax_rate_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.tax_rate_id)?)
            },
            metadata: if req.metadata.is_empty() {
                None
            } else {
                serde_json::from_str(&req.metadata).ok()
            },
        };

        let plan = self.db.create_plan(&input).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create plan");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        // Create usage components
        let mut components = Vec::new();
        for comp in req.usage_components {
            let comp_input = CreateUsageComponent {
                plan_id: plan.plan_id,
                name: comp.name,
                unit_name: comp.unit_name,
                unit_price: parse_decimal(&comp.unit_price)?,
                included_units: comp.included_units,
            };
            let component = self
                .db
                .create_usage_component(&comp_input)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to create usage component");
                    record_error("database", method);
                    record_grpc_request(method, "error");
                    record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                    Status::internal(e.to_string())
                })?;
            components.push(component);
        }

        record_plan_operation(&tenant_id.to_string(), "created");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(CreatePlanResponse {
            plan: Some(plan_to_proto(plan, components)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetPlan"))]
    async fn get_plan(
        &self,
        request: Request<GetPlanRequest>,
    ) -> Result<Response<GetPlanResponse>, Status> {
        let start = Instant::now();
        let method = "GetPlan";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_PLAN_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let plan_id = parse_uuid(&req.plan_id)?;

        tracing::debug!(tenant_id = %tenant_id, plan_id = %plan_id, "Getting plan");

        let plan = self.db.get_plan(tenant_id, plan_id).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get plan");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        let plan = plan.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Plan not found")
        })?;
        let components = self
            .db
            .get_usage_components(plan.plan_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get usage components");
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetPlanResponse {
            plan: Some(plan_to_proto(plan, components)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "UpdatePlan"))]
    async fn update_plan(
        &self,
        request: Request<UpdatePlanRequest>,
    ) -> Result<Response<UpdatePlanResponse>, Status> {
        let start = Instant::now();
        let method = "UpdatePlan";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_PLAN_UPDATE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let plan_id = parse_uuid(&req.plan_id)?;

        tracing::info!(tenant_id = %tenant_id, plan_id = %plan_id, "Updating plan");

        let input = UpdatePlan {
            name: if req.name.is_empty() {
                None
            } else {
                Some(req.name)
            },
            description: if req.description.is_empty() {
                None
            } else {
                Some(req.description)
            },
            base_price: if req.base_price.is_empty() {
                None
            } else {
                Some(parse_decimal(&req.base_price)?)
            },
            tax_rate_id: if req.tax_rate_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.tax_rate_id)?)
            },
            metadata: if req.metadata.is_empty() {
                None
            } else {
                serde_json::from_str(&req.metadata).ok()
            },
        };

        let plan = self
            .db
            .update_plan(tenant_id, plan_id, &input)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update plan");
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let plan = plan.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Plan not found or archived")
        })?;
        let components = self
            .db
            .get_usage_components(plan.plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_plan_operation(&tenant_id.to_string(), "updated");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(UpdatePlanResponse {
            plan: Some(plan_to_proto(plan, components)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListPlans"))]
    async fn list_plans(
        &self,
        request: Request<ListPlansRequest>,
    ) -> Result<Response<ListPlansResponse>, Status> {
        let start = Instant::now();
        let method = "ListPlans";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_PLAN_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        tracing::debug!(tenant_id = %tenant_id, "Listing plans");

        let filter = ListPlansFilter {
            include_archived: req.include_archived,
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let plans = self.db.list_plans(tenant_id, &filter).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to list plans");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        let mut proto_plans = Vec::new();
        for plan in plans {
            let components = self
                .db
                .get_usage_components(plan.plan_id)
                .await
                .map_err(|e| {
                    record_error("database", method);
                    record_grpc_request(method, "error");
                    record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                    Status::internal(e.to_string())
                })?;
            proto_plans.push(plan_to_proto(plan, components));
        }

        let next_page_token = proto_plans
            .last()
            .map(|p| p.plan_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListPlansResponse {
            plans: proto_plans,
            next_page_token,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ArchivePlan"))]
    async fn archive_plan(
        &self,
        request: Request<ArchivePlanRequest>,
    ) -> Result<Response<ArchivePlanResponse>, Status> {
        let start = Instant::now();
        let method = "ArchivePlan";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_PLAN_UPDATE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let plan_id = parse_uuid(&req.plan_id)?;

        tracing::info!(tenant_id = %tenant_id, plan_id = %plan_id, "Archiving plan");

        let plan = self
            .db
            .archive_plan(tenant_id, plan_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to archive plan");
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let plan = plan.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Plan not found or already archived")
        })?;
        let components = self
            .db
            .get_usage_components(plan.plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_plan_operation(&tenant_id.to_string(), "archived");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ArchivePlanResponse {
            plan: Some(plan_to_proto(plan, components)),
        }))
    }

    // =========================================================================
    // Subscription Management
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "CreateSubscription"))]
    async fn create_subscription(
        &self,
        request: Request<CreateSubscriptionRequest>,
    ) -> Result<Response<CreateSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "CreateSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_CREATE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        tracing::info!(
            tenant_id = %tenant_id,
            customer_id = %req.customer_id,
            plan_id = %req.plan_id,
            "Creating subscription"
        );

        let start_date = if req.start_date.is_empty() {
            Utc::now().date_naive()
        } else {
            parse_date(&req.start_date)?
        };

        let input = CreateSubscription {
            tenant_id,
            customer_id: parse_uuid(&req.customer_id)?,
            plan_id: parse_uuid(&req.plan_id)?,
            billing_anchor_day: if req.billing_anchor_day > 0 && req.billing_anchor_day <= 31 {
                req.billing_anchor_day
            } else {
                start_date.day() as i32
            },
            start_date,
            trial_end_date: if req.trial_end_date.is_empty() {
                None
            } else {
                Some(parse_date(&req.trial_end_date)?)
            },
            proration_mode: ProrationMode::from_proto(req.proration_mode),
            metadata: if req.metadata.is_empty() {
                None
            } else {
                serde_json::from_str(&req.metadata).ok()
            },
        };

        let subscription = self.db.create_subscription(&input).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create subscription");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            match e {
                service_core::error::AppError::NotFound(_) => Status::not_found("Plan not found"),
                _ => Status::internal(e.to_string()),
            }
        })?;

        // Create initial billing cycle
        let initial_cycle = self
            .db
            .create_billing_cycle(
                subscription.subscription_id,
                subscription.current_period_start,
                subscription.current_period_end,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create initial billing cycle");
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_subscription_operation(&tenant_id.to_string(), "created");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(CreateSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
            initial_cycle: Some(cycle_to_proto(initial_cycle, vec![])),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetSubscription"))]
    async fn get_subscription(
        &self,
        request: Request<GetSubscriptionRequest>,
    ) -> Result<Response<GetSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "GetSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::debug!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Getting subscription");

        let subscription = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        let current_cycle = self
            .db
            .get_current_billing_cycle(subscription.subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let cycle_proto = if let Some(cycle) = current_cycle {
            let charges = self
                .db
                .list_charges(tenant_id, cycle.cycle_id, &ListChargesFilter::default())
                .await
                .map_err(|e| {
                    record_error("database", method);
                    record_grpc_request(method, "error");
                    record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                    Status::internal(e.to_string())
                })?;
            Some(cycle_to_proto(
                cycle,
                charges.into_iter().map(charge_to_proto).collect(),
            ))
        } else {
            None
        };

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
            current_cycle: cycle_proto,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListSubscriptions"))]
    async fn list_subscriptions(
        &self,
        request: Request<ListSubscriptionsRequest>,
    ) -> Result<Response<ListSubscriptionsResponse>, Status> {
        let start = Instant::now();
        let method = "ListSubscriptions";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        tracing::debug!(tenant_id = %tenant_id, "Listing subscriptions");

        let filter = ListSubscriptionsFilter {
            customer_id: if req.customer_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.customer_id)?)
            },
            status: if req.status == 0 {
                None
            } else {
                Some(SubscriptionStatus::from_proto(req.status))
            },
            plan_id: if req.plan_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.plan_id)?)
            },
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let subscriptions = self
            .db
            .list_subscriptions(tenant_id, &filter)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_subscriptions: Vec<_> = subscriptions
            .into_iter()
            .map(subscription_to_proto)
            .collect();
        let next_page_token = proto_subscriptions
            .last()
            .map(|s| s.subscription_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListSubscriptionsResponse {
            subscriptions: proto_subscriptions,
            next_page_token,
        }))
    }

    // =========================================================================
    // Subscription Lifecycle
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "ActivateSubscription"))]
    async fn activate_subscription(
        &self,
        request: Request<ActivateSubscriptionRequest>,
    ) -> Result<Response<ActivateSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "ActivateSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Activating subscription");

        // Verify subscription is in trial status
        let existing = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let existing = existing.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if existing.status != SubscriptionStatus::Trial.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Subscription must be in trial status to activate",
            ));
        }

        let subscription = self
            .db
            .update_subscription_status(
                tenant_id,
                subscription_id,
                SubscriptionStatus::Active,
                None,
            )
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to update subscription")
        })?;

        record_subscription_operation(&tenant_id.to_string(), "activated");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ActivateSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "PauseSubscription"))]
    async fn pause_subscription(
        &self,
        request: Request<PauseSubscriptionRequest>,
    ) -> Result<Response<PauseSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "PauseSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Pausing subscription");

        // Verify subscription is active
        let existing = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let existing = existing.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if existing.status != SubscriptionStatus::Active.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Subscription must be active to pause",
            ));
        }

        let subscription = self
            .db
            .update_subscription_status(
                tenant_id,
                subscription_id,
                SubscriptionStatus::Paused,
                None,
            )
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to update subscription")
        })?;

        record_subscription_operation(&tenant_id.to_string(), "paused");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(PauseSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ResumeSubscription"))]
    async fn resume_subscription(
        &self,
        request: Request<ResumeSubscriptionRequest>,
    ) -> Result<Response<ResumeSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "ResumeSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Resuming subscription");

        // Verify subscription is paused
        let existing = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let existing = existing.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if existing.status != SubscriptionStatus::Paused.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Subscription must be paused to resume",
            ));
        }

        let subscription = self
            .db
            .update_subscription_status(
                tenant_id,
                subscription_id,
                SubscriptionStatus::Active,
                None,
            )
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to update subscription")
        })?;

        record_subscription_operation(&tenant_id.to_string(), "resumed");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ResumeSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "CancelSubscription"))]
    async fn cancel_subscription(
        &self,
        request: Request<CancelSubscriptionRequest>,
    ) -> Result<Response<CancelSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "CancelSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            cancel_at_period_end = %req.cancel_at_period_end,
            "Cancelling subscription"
        );

        // Verify subscription is not already cancelled
        let existing = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let existing = existing.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if existing.status == SubscriptionStatus::Cancelled.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Subscription is already cancelled",
            ));
        }

        let end_date = if req.cancel_at_period_end {
            Some(existing.current_period_end)
        } else {
            Some(Utc::now().date_naive())
        };

        let status = if req.cancel_at_period_end {
            // Keep current status until period end
            SubscriptionStatus::from_string(&existing.status)
        } else {
            SubscriptionStatus::Cancelled
        };

        let subscription = self
            .db
            .update_subscription_status(tenant_id, subscription_id, status, end_date)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to update subscription")
        })?;

        record_subscription_operation(&tenant_id.to_string(), "cancelled");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(CancelSubscriptionResponse {
            subscription: Some(subscription_to_proto(subscription)),
        }))
    }

    // =========================================================================
    // Plan Changes
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "ChangePlan"))]
    async fn change_plan(
        &self,
        request: Request<ChangePlanRequest>,
    ) -> Result<Response<ChangePlanResponse>, Status> {
        let start = Instant::now();
        let method = "ChangePlan";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_CHANGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;
        let new_plan_id = parse_uuid(&req.new_plan_id)?;

        tracing::info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            new_plan_id = %new_plan_id,
            "Changing plan"
        );

        // Validate new plan exists and is not archived
        let new_plan = self
            .db
            .get_plan(tenant_id, new_plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let new_plan = new_plan.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("New plan not found")
        })?;

        if new_plan.is_archived {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Cannot change to archived plan",
            ));
        }

        // Get current subscription
        let existing = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let existing = existing.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if existing.status != SubscriptionStatus::Active.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition(
                "Subscription must be active to change plan",
            ));
        }

        // Validate currency matches
        let old_plan = self
            .db
            .get_plan(tenant_id, existing.plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let old_plan = old_plan.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Current plan not found")
        })?;

        if old_plan.currency != new_plan.currency {
            record_grpc_request(method, "invalid_argument");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::invalid_argument(
                "Cannot change to plan with different currency",
            ));
        }

        let mode = if req.proration_mode == 0 {
            ProrationMode::from_string(&existing.proration_mode)
        } else {
            ProrationMode::from_proto(req.proration_mode)
        };

        let mut proration_charges = Vec::new();

        // Calculate proration charges for immediate mode
        if mode == ProrationMode::Immediate {
            let current_cycle = self
                .db
                .get_current_billing_cycle(subscription_id)
                .await
                .map_err(|e| {
                    record_error("database", method);
                    record_grpc_request(method, "error");
                    record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                    Status::internal(e.to_string())
                })?;

            if let Some(cycle) = current_cycle {
                let today = Utc::now().date_naive();
                let total_days = (cycle.period_end - cycle.period_start).num_days() as f64;
                let days_remaining = (cycle.period_end - today).num_days().max(0) as f64;
                let proration_factor =
                    Decimal::from_f64_retain(days_remaining / total_days).unwrap_or(Decimal::ZERO);

                // Credit for unused old plan
                let old_credit = -(old_plan.base_price * proration_factor);
                // Charge for new plan remaining days
                let new_charge = new_plan.base_price * proration_factor;

                if old_credit != Decimal::ZERO {
                    let credit_input = CreateCharge {
                        cycle_id: cycle.cycle_id,
                        charge_type: ChargeType::Proration,
                        description: format!("Credit for unused {} plan", old_plan.name),
                        quantity: Decimal::ONE,
                        unit_price: old_credit,
                        amount: old_credit,
                        is_prorated: true,
                        proration_factor: Some(proration_factor),
                        component_id: None,
                        metadata: None,
                    };
                    self.db.create_charge(&credit_input).await.map_err(|e| {
                        record_error("database", method);
                        record_grpc_request(method, "error");
                        record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                        Status::internal(e.to_string())
                    })?;
                    proration_charges.push(ProrationCharge {
                        description: credit_input.description,
                        amount: old_credit.to_string(),
                    });
                }

                if new_charge != Decimal::ZERO {
                    let charge_input = CreateCharge {
                        cycle_id: cycle.cycle_id,
                        charge_type: ChargeType::Proration,
                        description: format!("Charge for {} plan (prorated)", new_plan.name),
                        quantity: Decimal::ONE,
                        unit_price: new_charge,
                        amount: new_charge,
                        is_prorated: true,
                        proration_factor: Some(proration_factor),
                        component_id: None,
                        metadata: None,
                    };
                    self.db.create_charge(&charge_input).await.map_err(|e| {
                        record_error("database", method);
                        record_grpc_request(method, "error");
                        record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                        Status::internal(e.to_string())
                    })?;
                    proration_charges.push(ProrationCharge {
                        description: charge_input.description,
                        amount: new_charge.to_string(),
                    });
                }
            }
        }

        let subscription = self
            .db
            .change_subscription_plan(tenant_id, subscription_id, new_plan_id, mode)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to change plan")
        })?;

        record_subscription_operation(&tenant_id.to_string(), "plan_changed");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ChangePlanResponse {
            subscription: Some(subscription_to_proto(subscription)),
            proration_charges,
        }))
    }

    // =========================================================================
    // Usage Tracking
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "RecordUsage"))]
    async fn record_usage(
        &self,
        request: Request<RecordUsageRequest>,
    ) -> Result<Response<RecordUsageResponse>, Status> {
        let start = Instant::now();
        let method = "RecordUsage";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_USAGE_WRITE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;
        let component_id = parse_uuid(&req.component_id)?;

        tracing::debug!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            component_id = %component_id,
            quantity = %req.quantity,
            "Recording usage"
        );

        // Validate subscription exists and belongs to tenant
        let subscription = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        subscription.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        let input = RecordUsage {
            subscription_id,
            component_id,
            quantity: parse_decimal(&req.quantity)?,
            timestamp: timestamp_to_datetime(req.timestamp),
            idempotency_key: req.idempotency_key,
            metadata: if req.metadata.is_empty() {
                None
            } else {
                serde_json::from_str(&req.metadata).ok()
            },
        };

        let record = self.db.record_usage(&input).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to record usage");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        record_usage_operation(&tenant_id.to_string(), &component_id.to_string());
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(RecordUsageResponse {
            usage_record: Some(usage_record_to_proto(record)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetUsage"))]
    async fn get_usage(
        &self,
        request: Request<GetUsageRequest>,
    ) -> Result<Response<GetUsageResponse>, Status> {
        let start = Instant::now();
        let method = "GetUsage";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_USAGE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let record_id = parse_uuid(&req.record_id)?;

        tracing::debug!(tenant_id = %tenant_id, record_id = %record_id, "Getting usage");

        let record = self
            .db
            .get_usage_record(tenant_id, record_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let record = record.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Usage record not found")
        })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetUsageResponse {
            usage_record: Some(usage_record_to_proto(record)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListUsage"))]
    async fn list_usage(
        &self,
        request: Request<ListUsageRequest>,
    ) -> Result<Response<ListUsageResponse>, Status> {
        let start = Instant::now();
        let method = "ListUsage";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_USAGE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::debug!(tenant_id = %tenant_id, "Listing usage");

        let filter = ListUsageFilter {
            component_id: if req.component_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.component_id)?)
            },
            cycle_id: if req.cycle_id.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.cycle_id)?)
            },
            is_invoiced: if req.is_invoiced { Some(true) } else { None },
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let records = self
            .db
            .list_usage_records(tenant_id, subscription_id, &filter)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_records: Vec<_> = records.into_iter().map(usage_record_to_proto).collect();
        let next_page_token = proto_records
            .last()
            .map(|r| r.record_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListUsageResponse {
            usage_records: proto_records,
            next_page_token,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetUsageSummary"))]
    async fn get_usage_summary(
        &self,
        request: Request<GetUsageSummaryRequest>,
    ) -> Result<Response<GetUsageSummaryResponse>, Status> {
        let start = Instant::now();
        let method = "GetUsageSummary";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_USAGE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;
        let cycle_id = if req.cycle_id.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.cycle_id)?)
        };

        tracing::debug!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            "Getting usage summary"
        );

        let summaries = self
            .db
            .get_usage_summary(tenant_id, subscription_id, cycle_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_summaries: Vec<_> = summaries
            .into_iter()
            .map(|s| UsageComponentSummary {
                component_id: s.component_id.to_string(),
                name: s.name,
                total_quantity: s.total_quantity.to_string(),
                included_units: s.included_units,
                billable_units: s.billable_units.to_string(),
                amount: s.amount.to_string(),
            })
            .collect();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetUsageSummaryResponse {
            component_summaries: proto_summaries,
        }))
    }

    // =========================================================================
    // Billing Cycles
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "GetBillingCycle"))]
    async fn get_billing_cycle(
        &self,
        request: Request<GetBillingCycleRequest>,
    ) -> Result<Response<GetBillingCycleResponse>, Status> {
        let start = Instant::now();
        let method = "GetBillingCycle";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CYCLE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let cycle_id = parse_uuid(&req.cycle_id)?;

        tracing::debug!(tenant_id = %tenant_id, cycle_id = %cycle_id, "Getting billing cycle");

        let cycle = self
            .db
            .get_billing_cycle(tenant_id, cycle_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let cycle = cycle.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Billing cycle not found")
        })?;

        let charges = self
            .db
            .list_charges(tenant_id, cycle_id, &ListChargesFilter::default())
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetBillingCycleResponse {
            billing_cycle: Some(cycle_to_proto(
                cycle,
                charges.into_iter().map(charge_to_proto).collect(),
            )),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListBillingCycles"))]
    async fn list_billing_cycles(
        &self,
        request: Request<ListBillingCyclesRequest>,
    ) -> Result<Response<ListBillingCyclesResponse>, Status> {
        let start = Instant::now();
        let method = "ListBillingCycles";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CYCLE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::debug!(tenant_id = %tenant_id, "Listing billing cycles");

        let filter = ListBillingCyclesFilter {
            status: if req.status == 0 {
                None
            } else {
                Some(BillingCycleStatus::from_proto(req.status))
            },
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let cycles = self
            .db
            .list_billing_cycles(tenant_id, subscription_id, &filter)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_cycles: Vec<_> = cycles
            .into_iter()
            .map(|c| cycle_to_proto(c, vec![]))
            .collect();
        let next_page_token = proto_cycles
            .last()
            .map(|c| c.cycle_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListBillingCyclesResponse {
            billing_cycles: proto_cycles,
            next_page_token,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "AdvanceBillingCycle"))]
    async fn advance_billing_cycle(
        &self,
        request: Request<AdvanceBillingCycleRequest>,
    ) -> Result<Response<AdvanceBillingCycleResponse>, Status> {
        let start = Instant::now();
        let method = "AdvanceBillingCycle";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CYCLE_MANAGE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            "Advancing billing cycle"
        );

        let (previous_cycle, new_cycle) = self
            .db
            .advance_billing_cycle(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to advance billing cycle");
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                match e {
                    service_core::error::AppError::NotFound(_) => Status::not_found(e.to_string()),
                    service_core::error::AppError::BadRequest(_) => {
                        Status::failed_precondition(e.to_string())
                    }
                    _ => Status::internal(e.to_string()),
                }
            })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(AdvanceBillingCycleResponse {
            previous_cycle: Some(cycle_to_proto(previous_cycle, vec![])),
            new_cycle: Some(cycle_to_proto(new_cycle, vec![])),
        }))
    }

    // =========================================================================
    // Charges
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "GetCharge"))]
    async fn get_charge(
        &self,
        request: Request<GetChargeRequest>,
    ) -> Result<Response<GetChargeResponse>, Status> {
        let start = Instant::now();
        let method = "GetCharge";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CYCLE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let charge_id = parse_uuid(&req.charge_id)?;

        tracing::debug!(tenant_id = %tenant_id, charge_id = %charge_id, "Getting charge");

        let charge = self
            .db
            .get_charge(tenant_id, charge_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let charge = charge.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Charge not found")
        })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetChargeResponse {
            charge: Some(charge_to_proto(charge)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListCharges"))]
    async fn list_charges(
        &self,
        request: Request<ListChargesRequest>,
    ) -> Result<Response<ListChargesResponse>, Status> {
        let start = Instant::now();
        let method = "ListCharges";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CYCLE_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let cycle_id = parse_uuid(&req.cycle_id)?;

        tracing::debug!(tenant_id = %tenant_id, "Listing charges");

        let filter = ListChargesFilter {
            charge_type: if req.charge_type == 0 {
                None
            } else {
                Some(ChargeType::from_proto(req.charge_type))
            },
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let charges = self
            .db
            .list_charges(tenant_id, cycle_id, &filter)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_charges: Vec<_> = charges.into_iter().map(charge_to_proto).collect();
        let next_page_token = proto_charges
            .last()
            .map(|c| c.charge_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListChargesResponse {
            charges: proto_charges,
            next_page_token,
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "CreateOneTimeCharge"))]
    async fn create_one_time_charge(
        &self,
        request: Request<CreateOneTimeChargeRequest>,
    ) -> Result<Response<CreateOneTimeChargeResponse>, Status> {
        let start = Instant::now();
        let method = "CreateOneTimeCharge";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_CHARGE_CREATE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            amount = %req.amount,
            "Creating one-time charge"
        );

        // Validate subscription exists and is active
        let subscription = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                Status::internal(e.to_string())
            })?;

        let subscription =
            subscription.ok_or_else(|| Status::not_found("Subscription not found"))?;

        if subscription.status != SubscriptionStatus::Active.as_str() {
            return Err(Status::failed_precondition(
                "Subscription must be active to add charges",
            ));
        }

        // Get plan for currency
        let plan = self
            .db
            .get_plan(tenant_id, subscription.plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                Status::internal(e.to_string())
            })?;
        let currency = plan
            .map(|p| p.currency)
            .unwrap_or_else(|| "USD".to_string());

        // Get current billing cycle
        let cycle = self
            .db
            .get_current_billing_cycle(subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                Status::internal(e.to_string())
            })?;

        let cycle = cycle.ok_or_else(|| Status::failed_precondition("No pending billing cycle"))?;

        let amount = parse_decimal(&req.amount)?;

        let input = CreateCharge {
            cycle_id: cycle.cycle_id,
            charge_type: ChargeType::OneTime,
            description: req.description,
            quantity: Decimal::ONE,
            unit_price: amount,
            amount,
            is_prorated: false,
            proration_factor: None,
            component_id: None,
            metadata: if req.metadata.is_empty() {
                None
            } else {
                serde_json::from_str(&req.metadata).ok()
            },
        };

        let charge = self.db.create_charge(&input).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create charge");
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        // Track monetary amount for financial reporting
        if let Some(amount_f64) = amount.to_f64() {
            record_charge_amount(&tenant_id.to_string(), &currency, "one_time", amount_f64);
        }

        record_charge_created(&tenant_id.to_string(), "one_time");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(CreateOneTimeChargeResponse {
            charge: Some(charge_to_proto(charge)),
        }))
    }

    // =========================================================================
    // Billing Runs
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "RunBilling"))]
    async fn run_billing(
        &self,
        request: Request<RunBillingRequest>,
    ) -> Result<Response<RunBillingResponse>, Status> {
        let start = Instant::now();
        let method = "RunBilling";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_RUN_EXECUTE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let run_type = BillingRunType::from_proto(req.run_type);

        tracing::info!(tenant_id = %tenant_id, run_type = ?run_type, "Starting billing run");

        // Create billing run record
        let billing_run = self
            .db
            .create_billing_run(tenant_id, run_type)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        // Find subscriptions due for billing
        let subscriptions = self
            .db
            .find_subscriptions_due_for_billing(tenant_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let mut processed = 0;
        let mut succeeded = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for subscription in subscriptions {
            processed += 1;

            // Get current billing cycle
            let cycle = match self
                .db
                .get_current_billing_cycle(subscription.subscription_id)
                .await
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    failed += 1;
                    let result = self
                        .db
                        .create_billing_run_result(
                            billing_run.run_id,
                            subscription.subscription_id,
                            "failed",
                            None,
                            Some("No pending billing cycle".to_string()),
                        )
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;
                    results.push(billing_run_result_to_proto(result));
                    continue;
                }
                Err(e) => {
                    failed += 1;
                    let result = self
                        .db
                        .create_billing_run_result(
                            billing_run.run_id,
                            subscription.subscription_id,
                            "failed",
                            None,
                            Some(e.to_string()),
                        )
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;
                    results.push(billing_run_result_to_proto(result));
                    continue;
                }
            };

            // Get plan for recurring charge
            let plan = match self.db.get_plan(tenant_id, subscription.plan_id).await {
                Ok(Some(p)) => p,
                Ok(None) | Err(_) => {
                    failed += 1;
                    let result = self
                        .db
                        .create_billing_run_result(
                            billing_run.run_id,
                            subscription.subscription_id,
                            "failed",
                            None,
                            Some("Plan not found".to_string()),
                        )
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;
                    results.push(billing_run_result_to_proto(result));
                    continue;
                }
            };

            // Create recurring charge
            let recurring_input = CreateCharge {
                cycle_id: cycle.cycle_id,
                charge_type: ChargeType::Recurring,
                description: format!("Monthly subscription - {}", plan.name),
                quantity: Decimal::ONE,
                unit_price: plan.base_price,
                amount: plan.base_price,
                is_prorated: false,
                proration_factor: None,
                component_id: None,
                metadata: None,
            };

            if let Err(e) = self.db.create_charge(&recurring_input).await {
                tracing::error!(error = %e, "Failed to create recurring charge");
            }

            // Create usage charges
            let usage_summaries = self
                .db
                .get_usage_summary(
                    tenant_id,
                    subscription.subscription_id,
                    Some(cycle.cycle_id),
                )
                .await
                .unwrap_or_default();

            for summary in usage_summaries {
                if summary.billable_units > Decimal::ZERO {
                    let usage_input = CreateCharge {
                        cycle_id: cycle.cycle_id,
                        charge_type: ChargeType::Usage,
                        description: format!(
                            "{} - {} billable units",
                            summary.name, summary.billable_units
                        ),
                        quantity: summary.billable_units,
                        unit_price: summary.amount / summary.billable_units,
                        amount: summary.amount,
                        is_prorated: false,
                        proration_factor: None,
                        component_id: Some(summary.component_id),
                        metadata: None,
                    };

                    if let Err(e) = self.db.create_charge(&usage_input).await {
                        tracing::error!(error = %e, "Failed to create usage charge");
                    }
                }
            }

            // Mark cycle as invoiced (invoice creation would be via invoicing-service)
            if let Err(e) = self
                .db
                .update_billing_cycle_status(cycle.cycle_id, BillingCycleStatus::Invoiced, None)
                .await
            {
                tracing::error!(error = %e, "Failed to update cycle status");
            }

            // Mark usage as invoiced
            if let Err(e) = self.db.mark_usage_invoiced(cycle.cycle_id).await {
                tracing::error!(error = %e, "Failed to mark usage as invoiced");
            }

            succeeded += 1;
            let result = self
                .db
                .create_billing_run_result(
                    billing_run.run_id,
                    subscription.subscription_id,
                    "success",
                    None, // Invoice ID would come from invoicing-service
                    None,
                )
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            results.push(billing_run_result_to_proto(result));
        }

        // Update billing run with final status
        let status = if failed == 0 {
            BillingRunStatus::Completed
        } else if succeeded == 0 {
            BillingRunStatus::Failed
        } else {
            BillingRunStatus::Completed
        };

        let billing_run = self
            .db
            .update_billing_run(
                billing_run.run_id,
                status,
                processed,
                succeeded,
                failed,
                None,
            )
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let billing_run = billing_run.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Failed to update billing run")
        })?;

        record_billing_run(&tenant_id.to_string(), run_type.as_str(), status.as_str());
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(RunBillingResponse {
            billing_run: Some(billing_run_to_proto(billing_run, results)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "RunBillingForSubscription"))]
    #[tracing::instrument(skip(self, request), fields(method = "RunBillingForSubscription"))]
    async fn run_billing_for_subscription(
        &self,
        request: Request<RunBillingForSubscriptionRequest>,
    ) -> Result<Response<RunBillingForSubscriptionResponse>, Status> {
        let start = Instant::now();
        let method = "RunBillingForSubscription";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_RUN_EXECUTE)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let subscription_id = parse_uuid(&req.subscription_id)?;

        tracing::info!(
            tenant_id = %tenant_id,
            subscription_id = %subscription_id,
            "Running billing for subscription"
        );

        // Create billing run
        let billing_run = self
            .db
            .create_billing_run(tenant_id, BillingRunType::Single)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        // Get subscription
        let subscription = self
            .db
            .get_subscription(tenant_id, subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let subscription = subscription.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Subscription not found")
        })?;

        if subscription.status != SubscriptionStatus::Active.as_str() {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            return Err(Status::failed_precondition("Subscription must be active"));
        }

        // Get current billing cycle
        let cycle = self
            .db
            .get_current_billing_cycle(subscription_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let cycle = cycle.ok_or_else(|| {
            record_grpc_request(method, "failed_precondition");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::failed_precondition("No pending billing cycle")
        })?;

        // Get plan
        let plan = self
            .db
            .get_plan(tenant_id, subscription.plan_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let plan = plan.ok_or_else(|| {
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal("Plan not found")
        })?;

        // Create recurring charge
        let recurring_input = CreateCharge {
            cycle_id: cycle.cycle_id,
            charge_type: ChargeType::Recurring,
            description: format!("Monthly subscription - {}", plan.name),
            quantity: Decimal::ONE,
            unit_price: plan.base_price,
            amount: plan.base_price,
            is_prorated: false,
            proration_factor: None,
            component_id: None,
            metadata: None,
        };

        self.db.create_charge(&recurring_input).await.map_err(|e| {
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        // Create usage charges
        let usage_summaries = self
            .db
            .get_usage_summary(tenant_id, subscription_id, Some(cycle.cycle_id))
            .await
            .unwrap_or_default();

        for summary in usage_summaries {
            if summary.billable_units > Decimal::ZERO {
                let usage_input = CreateCharge {
                    cycle_id: cycle.cycle_id,
                    charge_type: ChargeType::Usage,
                    description: format!(
                        "{} - {} billable units",
                        summary.name, summary.billable_units
                    ),
                    quantity: summary.billable_units,
                    unit_price: summary.amount / summary.billable_units,
                    amount: summary.amount,
                    is_prorated: false,
                    proration_factor: None,
                    component_id: Some(summary.component_id),
                    metadata: None,
                };

                self.db.create_charge(&usage_input).await.map_err(|e| {
                    record_error("database", method);
                    record_grpc_request(method, "error");
                    record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                    Status::internal(e.to_string())
                })?;
            }
        }

        // Update cycle status
        self.db
            .update_billing_cycle_status(cycle.cycle_id, BillingCycleStatus::Invoiced, None)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        // Mark usage as invoiced
        self.db
            .mark_usage_invoiced(cycle.cycle_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        // Create result
        let result = self
            .db
            .create_billing_run_result(billing_run.run_id, subscription_id, "success", None, None)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        // Update billing run
        self.db
            .update_billing_run(
                billing_run.run_id,
                BillingRunStatus::Completed,
                1,
                1,
                0,
                None,
            )
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        record_billing_run(&tenant_id.to_string(), "single", "completed");
        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(RunBillingForSubscriptionResponse {
            result: Some(billing_run_result_to_proto(result)),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetBillingRun"))]
    async fn get_billing_run(
        &self,
        request: Request<GetBillingRunRequest>,
    ) -> Result<Response<GetBillingRunResponse>, Status> {
        let start = Instant::now();
        let method = "GetBillingRun";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_RUN_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        let run_id = parse_uuid(&req.run_id)?;

        tracing::debug!(tenant_id = %tenant_id, run_id = %run_id, "Getting billing run");

        let billing_run = self
            .db
            .get_billing_run(tenant_id, run_id)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let billing_run = billing_run.ok_or_else(|| {
            record_grpc_request(method, "not_found");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::not_found("Billing run not found")
        })?;

        let results = self.db.get_billing_run_results(run_id).await.map_err(|e| {
            record_error("database", method);
            record_grpc_request(method, "error");
            record_grpc_request_duration(method, start.elapsed().as_secs_f64());
            Status::internal(e.to_string())
        })?;

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(GetBillingRunResponse {
            billing_run: Some(billing_run_to_proto(
                billing_run,
                results
                    .into_iter()
                    .map(billing_run_result_to_proto)
                    .collect(),
            )),
        }))
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListBillingRuns"))]
    async fn list_billing_runs(
        &self,
        request: Request<ListBillingRunsRequest>,
    ) -> Result<Response<ListBillingRunsResponse>, Status> {
        let start = Instant::now();
        let method = "ListBillingRuns";

        let auth = self
            .capability_checker
            .require_capability(&request, capabilities::BILLING_RUN_READ)
            .await?;
        let tenant_id = parse_tenant_id(&auth)?;

        let req = request.into_inner();
        tracing::debug!(tenant_id = %tenant_id, "Listing billing runs");

        let filter = ListBillingRunsFilter {
            status: if req.status == 0 {
                None
            } else {
                Some(BillingRunStatus::from_proto(req.status))
            },
            run_type: if req.run_type == 0 {
                None
            } else {
                Some(BillingRunType::from_proto(req.run_type))
            },
            page_size: if req.page_size > 0 { req.page_size } else { 50 },
            page_token: if req.page_token.is_empty() {
                None
            } else {
                Some(parse_uuid(&req.page_token)?)
            },
        };

        let runs = self
            .db
            .list_billing_runs(tenant_id, &filter)
            .await
            .map_err(|e| {
                record_error("database", method);
                record_grpc_request(method, "error");
                record_grpc_request_duration(method, start.elapsed().as_secs_f64());
                Status::internal(e.to_string())
            })?;

        let proto_runs: Vec<_> = runs
            .into_iter()
            .map(|r| billing_run_to_proto(r, vec![]))
            .collect();
        let next_page_token = proto_runs
            .last()
            .map(|r| r.run_id.clone())
            .unwrap_or_default();

        record_grpc_request(method, "ok");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());

        Ok(Response::new(ListBillingRunsResponse {
            billing_runs: proto_runs,
            next_page_token,
        }))
    }
}
