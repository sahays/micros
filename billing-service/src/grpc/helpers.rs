//! Helper functions for type conversions between domain models and protobuf types.

use crate::grpc::proto::*;
use crate::models::{
    BillingCycleStatus, BillingInterval, BillingRunStatus, BillingRunType, ChargeType,
    ProrationMode, SubscriptionStatus,
};
use chrono::Utc;
use prost_types::Timestamp;
use rust_decimal::Decimal;
use std::str::FromStr;
use tonic::Status;
use uuid::Uuid;

#[allow(clippy::result_large_err)]
pub fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("Invalid UUID: {}", s)))
}

#[allow(clippy::result_large_err)]
pub fn parse_decimal(s: &str) -> Result<Decimal, Status> {
    Decimal::from_str(s).map_err(|_| Status::invalid_argument(format!("Invalid decimal: {}", s)))
}

#[allow(clippy::result_large_err)]
pub fn parse_date(s: &str) -> Result<chrono::NaiveDate, Status> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| Status::invalid_argument(format!("Invalid date: {}", s)))
}

pub fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

pub fn timestamp_to_datetime(ts: Option<Timestamp>) -> chrono::DateTime<Utc> {
    ts.map(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32).unwrap_or_else(Utc::now))
        .unwrap_or_else(Utc::now)
}

pub fn plan_to_proto(
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

pub fn component_to_proto(c: crate::models::UsageComponent) -> UsageComponent {
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

pub fn subscription_to_proto(s: crate::models::Subscription) -> Subscription {
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

pub fn cycle_to_proto(c: crate::models::BillingCycle, charges: Vec<Charge>) -> BillingCycle {
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

pub fn charge_to_proto(c: crate::models::Charge) -> Charge {
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

pub fn usage_record_to_proto(r: crate::models::UsageRecord) -> UsageRecord {
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

pub fn billing_run_to_proto(
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

pub fn billing_run_result_to_proto(r: crate::models::BillingRunResult) -> BillingRunResult {
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
