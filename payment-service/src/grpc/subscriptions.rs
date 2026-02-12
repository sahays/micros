//! gRPC handlers for subscription operations.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    check_feature_flag, check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_subscriptions;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn create_razorpay_plan(
    state: &AppState,
    request: Request<CreateRazorpayPlanRequest>,
) -> Result<Response<CreateRazorpayPlanResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_PLAN_CREATE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let period_str = match PlanPeriod::try_from(req.period) {
        Ok(PlanPeriod::Daily) => "daily",
        Ok(PlanPeriod::Weekly) => "weekly",
        Ok(PlanPeriod::Monthly) => "monthly",
        Ok(PlanPeriod::Yearly) => "yearly",
        _ => return Err(Status::invalid_argument("Invalid plan period")),
    };

    let rz_request = razorpay_subscriptions::CreatePlanRequest {
        period: period_str.to_string(),
        interval: req.interval,
        item: razorpay_subscriptions::PlanItem {
            name: req.name.clone(),
            description: req.description.clone(),
            amount: req.amount,
            currency: req.currency.clone(),
        },
    };

    let rz_response = state.razorpay.create_plan(rz_request).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create Razorpay plan");
        Status::internal(format!("Failed to create plan: {}", e))
    })?;

    let now = DateTime::now();
    let plan = models::RazorpayPlan {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_plan_id: rz_response.id,
        name: req.name,
        description: req.description,
        amount: req.amount,
        currency: req.currency,
        period: proto_to_plan_period(req.period).unwrap_or(models::PlanPeriod::Monthly),
        interval: req.interval,
        created_at: now,
    };

    state
        .repository
        .create_plan(plan.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save plan");
            Status::internal("Failed to save plan")
        })?;

    Ok(Response::new(CreateRazorpayPlanResponse {
        plan: Some(plan_to_proto(plan)),
    }))
}

pub async fn get_razorpay_plan(
    state: &AppState,
    request: Request<GetRazorpayPlanRequest>,
) -> Result<Response<GetRazorpayPlanResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_PLAN_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let plan = state
        .repository
        .get_plan_in_tenant(&tenant.app_id, &tenant.org_id, &req.plan_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch plan");
            Status::internal("Failed to fetch plan")
        })?
        .ok_or_else(|| Status::not_found("Plan not found"))?;

    Ok(Response::new(GetRazorpayPlanResponse {
        plan: Some(plan_to_proto(plan)),
    }))
}

pub async fn list_razorpay_plans(
    state: &AppState,
    request: Request<ListRazorpayPlansRequest>,
) -> Result<Response<ListRazorpayPlansResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_PLAN_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (plans, total_count) = state
        .repository
        .list_plans_in_tenant(&tenant.app_id, &tenant.org_id, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list plans");
            Status::internal("Failed to list plans")
        })?;

    Ok(Response::new(ListRazorpayPlansResponse {
        plans: plans.into_iter().map(plan_to_proto).collect(),
        total_count,
    }))
}

pub async fn create_razorpay_subscription(
    state: &AppState,
    request: Request<CreateRazorpaySubscriptionRequest>,
) -> Result<Response<CreateRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_CREATE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Validate plan exists
    let plan = state
        .repository
        .get_plan_in_tenant(&tenant.app_id, &tenant.org_id, &req.plan_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch plan");
            Status::internal("Failed to fetch plan")
        })?
        .ok_or_else(|| Status::not_found("Plan not found"))?;

    let notes: Option<serde_json::Value> = req
        .notes_json
        .as_ref()
        .and_then(|json| serde_json::from_str(json).ok());

    let rz_request = razorpay_subscriptions::CreateSubscriptionRequest {
        plan_id: plan.razorpay_plan_id,
        total_count: req.total_count,
        customer_id: req.customer_id.clone(),
        notes,
    };

    let rz_response = state
        .razorpay
        .create_subscription(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay subscription");
            Status::internal(format!("Failed to create subscription: {}", e))
        })?;

    let now = DateTime::now();
    let subscription = models::RazorpaySubscription {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_subscription_id: rz_response.id,
        plan_id: req.plan_id,
        customer_id: req.customer_id,
        status: models::SubscriptionStatus::Created,
        total_count: rz_response.total_count.unwrap_or(req.total_count),
        paid_count: rz_response.paid_count.unwrap_or(0) as i32,
        remaining_count: rz_response.remaining_count.unwrap_or(req.total_count) as i32,
        short_url: rz_response.short_url,
        charge_count: 0,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_subscription(subscription.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save subscription");
            Status::internal("Failed to save subscription")
        })?;

    Ok(Response::new(CreateRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(subscription)),
    }))
}

pub async fn get_razorpay_subscription(
    state: &AppState,
    request: Request<GetRazorpaySubscriptionRequest>,
) -> Result<Response<GetRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let sub = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    Ok(Response::new(GetRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(sub)),
    }))
}

pub async fn list_razorpay_subscriptions(
    state: &AppState,
    request: Request<ListRazorpaySubscriptionsRequest>,
) -> Result<Response<ListRazorpaySubscriptionsResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_subscription_status);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (subs, total_count) = state
        .repository
        .list_subscriptions_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            req.customer_id.as_deref(),
            req.plan_id.as_deref(),
            status_filter,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list subscriptions");
            Status::internal("Failed to list subscriptions")
        })?;

    Ok(Response::new(ListRazorpaySubscriptionsResponse {
        subscriptions: subs.into_iter().map(subscription_to_proto).collect(),
        total_count,
    }))
}

pub async fn pause_razorpay_subscription(
    state: &AppState,
    request: Request<PauseRazorpaySubscriptionRequest>,
) -> Result<Response<PauseRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_MANAGE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let sub = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    state
        .razorpay
        .pause_subscription(&sub.razorpay_subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to pause subscription");
            Status::internal(format!("Failed to pause subscription: {}", e))
        })?;

    state
        .repository
        .update_subscription_status_by_razorpay_id(
            &sub.razorpay_subscription_id,
            models::SubscriptionStatus::Paused,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update subscription status");
            Status::internal("Failed to update subscription")
        })?;

    let updated = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    Ok(Response::new(PauseRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(updated)),
    }))
}

pub async fn resume_razorpay_subscription(
    state: &AppState,
    request: Request<ResumeRazorpaySubscriptionRequest>,
) -> Result<Response<ResumeRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_MANAGE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let sub = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    state
        .razorpay
        .resume_subscription(&sub.razorpay_subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to resume subscription");
            Status::internal(format!("Failed to resume subscription: {}", e))
        })?;

    state
        .repository
        .update_subscription_status_by_razorpay_id(
            &sub.razorpay_subscription_id,
            models::SubscriptionStatus::Active,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update subscription status");
            Status::internal("Failed to update subscription")
        })?;

    let updated = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    Ok(Response::new(ResumeRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(updated)),
    }))
}

pub async fn cancel_razorpay_subscription(
    state: &AppState,
    request: Request<CancelRazorpaySubscriptionRequest>,
) -> Result<Response<CancelRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_MANAGE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let sub = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    state
        .razorpay
        .cancel_subscription(&sub.razorpay_subscription_id, req.cancel_at_cycle_end)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to cancel subscription");
            Status::internal(format!("Failed to cancel subscription: {}", e))
        })?;

    state
        .repository
        .update_subscription_status_by_razorpay_id(
            &sub.razorpay_subscription_id,
            models::SubscriptionStatus::Cancelled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update subscription status");
            Status::internal("Failed to update subscription")
        })?;

    let updated = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    Ok(Response::new(CancelRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(updated)),
    }))
}

pub async fn update_razorpay_subscription(
    state: &AppState,
    request: Request<UpdateRazorpaySubscriptionRequest>,
) -> Result<Response<UpdateRazorpaySubscriptionResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_SUBSCRIPTION_MANAGE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_subscriptions_enabled,
        "Razorpay Subscriptions",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let sub = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    let rz_request = razorpay_subscriptions::UpdateSubscriptionRequest {
        plan_id: req.plan_id.clone(),
        total_count: req.total_count,
    };

    state
        .razorpay
        .update_subscription(&sub.razorpay_subscription_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update subscription");
            Status::internal(format!("Failed to update subscription: {}", e))
        })?;

    let mut update = mongodb::bson::doc! { "updated_at": DateTime::now() };
    if let Some(plan_id) = &req.plan_id {
        update.insert("plan_id", plan_id);
    }
    if let Some(total_count) = req.total_count {
        update.insert("total_count", total_count);
    }

    state
        .repository
        .update_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id, update)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update subscription");
            Status::internal("Failed to update subscription")
        })?;

    let updated = state
        .repository
        .get_subscription_in_tenant(&tenant.app_id, &tenant.org_id, &req.subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch subscription");
            Status::internal("Failed to fetch subscription")
        })?
        .ok_or_else(|| Status::not_found("Subscription not found"))?;

    Ok(Response::new(UpdateRazorpaySubscriptionResponse {
        subscription: Some(subscription_to_proto(updated)),
    }))
}

fn plan_to_proto(p: models::RazorpayPlan) -> RazorpayPlan {
    RazorpayPlan {
        id: p.id,
        app_id: p.app_id,
        org_id: p.org_id,
        razorpay_plan_id: p.razorpay_plan_id,
        name: p.name,
        description: p.description,
        amount: p.amount,
        currency: p.currency,
        period: plan_period_to_proto(&p.period).into(),
        interval: p.interval,
        created_at: datetime_to_timestamp(p.created_at),
    }
}

fn subscription_to_proto(s: models::RazorpaySubscription) -> RazorpaySubscription {
    RazorpaySubscription {
        id: s.id,
        app_id: s.app_id,
        org_id: s.org_id,
        razorpay_subscription_id: s.razorpay_subscription_id,
        plan_id: s.plan_id,
        customer_id: s.customer_id,
        status: subscription_status_to_proto(&s.status).into(),
        total_count: s.total_count,
        paid_count: s.paid_count,
        remaining_count: s.remaining_count,
        short_url: s.short_url,
        charge_count: s.charge_count,
        created_at: datetime_to_timestamp(s.created_at),
        updated_at: datetime_to_timestamp(s.updated_at),
    }
}

fn plan_period_to_proto(p: &models::PlanPeriod) -> PlanPeriod {
    match p {
        models::PlanPeriod::Daily => PlanPeriod::Daily,
        models::PlanPeriod::Weekly => PlanPeriod::Weekly,
        models::PlanPeriod::Monthly => PlanPeriod::Monthly,
        models::PlanPeriod::Yearly => PlanPeriod::Yearly,
    }
}

fn proto_to_plan_period(p: i32) -> Option<models::PlanPeriod> {
    match PlanPeriod::try_from(p) {
        Ok(PlanPeriod::Daily) => Some(models::PlanPeriod::Daily),
        Ok(PlanPeriod::Weekly) => Some(models::PlanPeriod::Weekly),
        Ok(PlanPeriod::Monthly) => Some(models::PlanPeriod::Monthly),
        Ok(PlanPeriod::Yearly) => Some(models::PlanPeriod::Yearly),
        _ => None,
    }
}

fn subscription_status_to_proto(s: &models::SubscriptionStatus) -> SubscriptionStatus {
    match s {
        models::SubscriptionStatus::Created => SubscriptionStatus::Created,
        models::SubscriptionStatus::Authenticated => SubscriptionStatus::Authenticated,
        models::SubscriptionStatus::Active => SubscriptionStatus::Active,
        models::SubscriptionStatus::Pending => SubscriptionStatus::Pending,
        models::SubscriptionStatus::Halted => SubscriptionStatus::Halted,
        models::SubscriptionStatus::Paused => SubscriptionStatus::Paused,
        models::SubscriptionStatus::Cancelled => SubscriptionStatus::Cancelled,
        models::SubscriptionStatus::Completed => SubscriptionStatus::Completed,
        models::SubscriptionStatus::Expired => SubscriptionStatus::Expired,
    }
}

fn proto_to_subscription_status(s: i32) -> Option<models::SubscriptionStatus> {
    match SubscriptionStatus::try_from(s) {
        Ok(SubscriptionStatus::Created) => Some(models::SubscriptionStatus::Created),
        Ok(SubscriptionStatus::Authenticated) => Some(models::SubscriptionStatus::Authenticated),
        Ok(SubscriptionStatus::Active) => Some(models::SubscriptionStatus::Active),
        Ok(SubscriptionStatus::Pending) => Some(models::SubscriptionStatus::Pending),
        Ok(SubscriptionStatus::Halted) => Some(models::SubscriptionStatus::Halted),
        Ok(SubscriptionStatus::Paused) => Some(models::SubscriptionStatus::Paused),
        Ok(SubscriptionStatus::Cancelled) => Some(models::SubscriptionStatus::Cancelled),
        Ok(SubscriptionStatus::Completed) => Some(models::SubscriptionStatus::Completed),
        Ok(SubscriptionStatus::Expired) => Some(models::SubscriptionStatus::Expired),
        _ => None,
    }
}
