//! Subscription management gRPC handlers.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::{
    ChargeType, CreateCharge, CreateSubscription, ListChargesFilter, ListSubscriptionsFilter,
    ProrationMode, SubscriptionStatus,
};
use crate::services::Database;
use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn create_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<CreateSubscriptionRequest>,
) -> Result<Response<CreateSubscriptionResponse>, Status> {
    let auth = capability_checker
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

    let subscription = db.create_subscription(&input).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create subscription");
        match e {
            service_core::error::AppError::NotFound(_) => Status::not_found("Plan not found"),
            _ => Status::internal(e.to_string()),
        }
    })?;

    // Create initial billing cycle
    let initial_cycle = db
        .create_billing_cycle(
            subscription.subscription_id,
            subscription.current_period_start,
            subscription.current_period_end,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create initial billing cycle");
            Status::internal(e.to_string())
        })?;

    Ok(Response::new(CreateSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
        initial_cycle: Some(cycle_to_proto(initial_cycle, vec![])),
    }))
}

pub async fn get_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetSubscriptionRequest>,
) -> Result<Response<GetSubscriptionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::debug!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Getting subscription");

    let subscription = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    let current_cycle = db
        .get_current_billing_cycle(subscription.subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let cycle_proto = if let Some(cycle) = current_cycle {
        let charges = db
            .list_charges(tenant_id, cycle.cycle_id, &ListChargesFilter::default())
            .await
            .map_err(|e| {
                Status::internal(e.to_string())
            })?;
        Some(cycle_to_proto(
            cycle,
            charges.into_iter().map(charge_to_proto).collect(),
        ))
    } else {
        None
    };

    Ok(Response::new(GetSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
        current_cycle: cycle_proto,
    }))
}

pub async fn list_subscriptions(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListSubscriptionsRequest>,
) -> Result<Response<ListSubscriptionsResponse>, Status> {
    let auth = capability_checker
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

    let subscriptions = db
        .list_subscriptions(tenant_id, &filter)
        .await
        .map_err(|e| {
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

    Ok(Response::new(ListSubscriptionsResponse {
        subscriptions: proto_subscriptions,
        next_page_token,
    }))
}

pub async fn activate_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ActivateSubscriptionRequest>,
) -> Result<Response<ActivateSubscriptionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Activating subscription");

    // Verify subscription is in trial status
    let existing = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let existing = existing.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if existing.status != SubscriptionStatus::Trial.as_str() {
        return Err(Status::failed_precondition(
            "Subscription must be in trial status to activate",
        ));
    }

    let subscription = db
        .update_subscription_status(tenant_id, subscription_id, SubscriptionStatus::Active, None)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::internal("Failed to update subscription")
    })?;

    Ok(Response::new(ActivateSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
    }))
}

pub async fn pause_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<PauseSubscriptionRequest>,
) -> Result<Response<PauseSubscriptionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Pausing subscription");

    // Verify subscription is active
    let existing = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let existing = existing.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if existing.status != SubscriptionStatus::Active.as_str() {
        return Err(Status::failed_precondition(
            "Subscription must be active to pause",
        ));
    }

    let subscription = db
        .update_subscription_status(tenant_id, subscription_id, SubscriptionStatus::Paused, None)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::internal("Failed to update subscription")
    })?;

    Ok(Response::new(PauseSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
    }))
}

pub async fn resume_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ResumeSubscriptionRequest>,
) -> Result<Response<ResumeSubscriptionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_SUBSCRIPTION_MANAGE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::info!(tenant_id = %tenant_id, subscription_id = %subscription_id, "Resuming subscription");

    // Verify subscription is paused
    let existing = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let existing = existing.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if existing.status != SubscriptionStatus::Paused.as_str() {
        return Err(Status::failed_precondition(
            "Subscription must be paused to resume",
        ));
    }

    let subscription = db
        .update_subscription_status(tenant_id, subscription_id, SubscriptionStatus::Active, None)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::internal("Failed to update subscription")
    })?;

    Ok(Response::new(ResumeSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
    }))
}

pub async fn cancel_subscription(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<CancelSubscriptionRequest>,
) -> Result<Response<CancelSubscriptionResponse>, Status> {
    let auth = capability_checker
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
    let existing = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let existing = existing.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if existing.status == SubscriptionStatus::Cancelled.as_str() {
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

    let subscription = db
        .update_subscription_status(tenant_id, subscription_id, status, end_date)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::internal("Failed to update subscription")
    })?;

    Ok(Response::new(CancelSubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn change_plan(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ChangePlanRequest>,
) -> Result<Response<ChangePlanResponse>, Status> {
    let auth = capability_checker
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
    let new_plan = db.get_plan(tenant_id, new_plan_id).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    let new_plan = new_plan.ok_or_else(|| {
        Status::not_found("New plan not found")
    })?;

    if new_plan.is_archived {
        return Err(Status::failed_precondition(
            "Cannot change to archived plan",
        ));
    }

    // Get current subscription
    let existing = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let existing = existing.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if existing.status != SubscriptionStatus::Active.as_str() {
        return Err(Status::failed_precondition(
            "Subscription must be active to change plan",
        ));
    }

    // Validate currency matches
    let old_plan = db
        .get_plan(tenant_id, existing.plan_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let old_plan = old_plan.ok_or_else(|| {
        Status::internal("Current plan not found")
    })?;

    if old_plan.currency != new_plan.currency {
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
        let current_cycle = db
            .get_current_billing_cycle(subscription_id)
            .await
            .map_err(|e| {
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
                db.create_charge(&credit_input).await.map_err(|e| {
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
                db.create_charge(&charge_input).await.map_err(|e| {
                    Status::internal(e.to_string())
                })?;
                proration_charges.push(ProrationCharge {
                    description: charge_input.description,
                    amount: new_charge.to_string(),
                });
            }
        }
    }

    let subscription = db
        .change_subscription_plan(tenant_id, subscription_id, new_plan_id, mode)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::internal("Failed to change plan")
    })?;

    Ok(Response::new(ChangePlanResponse {
        subscription: Some(subscription_to_proto(subscription)),
        proration_charges,
    }))
}
