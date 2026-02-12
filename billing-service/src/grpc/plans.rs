//! Plan management gRPC handlers.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::BillingInterval;
use crate::models::{CreatePlan, CreateUsageComponent, ListPlansFilter, UpdatePlan};
use crate::services::{
    record_error, record_grpc_request, record_grpc_request_duration, record_plan_operation,
    Database,
};
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status};

pub async fn create_plan(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<CreatePlanRequest>,
) -> Result<Response<CreatePlanResponse>, Status> {
    let start = Instant::now();
    let method = "CreatePlan";

    let auth = capability_checker
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

    let plan = db.create_plan(&input).await.map_err(|e| {
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
        let component = db.create_usage_component(&comp_input).await.map_err(|e| {
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

pub async fn get_plan(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetPlanRequest>,
) -> Result<Response<GetPlanResponse>, Status> {
    let start = Instant::now();
    let method = "GetPlan";

    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_PLAN_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let plan_id = parse_uuid(&req.plan_id)?;

    tracing::debug!(tenant_id = %tenant_id, plan_id = %plan_id, "Getting plan");

    let plan = db.get_plan(tenant_id, plan_id).await.map_err(|e| {
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
    let components = db.get_usage_components(plan.plan_id).await.map_err(|e| {
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

pub async fn update_plan(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<UpdatePlanRequest>,
) -> Result<Response<UpdatePlanResponse>, Status> {
    let start = Instant::now();
    let method = "UpdatePlan";

    let auth = capability_checker
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

    let plan = db
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
    let components = db.get_usage_components(plan.plan_id).await.map_err(|e| {
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

pub async fn list_plans(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListPlansRequest>,
) -> Result<Response<ListPlansResponse>, Status> {
    let start = Instant::now();
    let method = "ListPlans";

    let auth = capability_checker
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

    let plans = db.list_plans(tenant_id, &filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list plans");
        record_error("database", method);
        record_grpc_request(method, "error");
        record_grpc_request_duration(method, start.elapsed().as_secs_f64());
        Status::internal(e.to_string())
    })?;

    let mut proto_plans = Vec::new();
    for plan in plans {
        let components = db.get_usage_components(plan.plan_id).await.map_err(|e| {
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

pub async fn archive_plan(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ArchivePlanRequest>,
) -> Result<Response<ArchivePlanResponse>, Status> {
    let start = Instant::now();
    let method = "ArchivePlan";

    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_PLAN_UPDATE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let plan_id = parse_uuid(&req.plan_id)?;

    tracing::info!(tenant_id = %tenant_id, plan_id = %plan_id, "Archiving plan");

    let plan = db.archive_plan(tenant_id, plan_id).await.map_err(|e| {
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
    let components = db.get_usage_components(plan.plan_id).await.map_err(|e| {
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
