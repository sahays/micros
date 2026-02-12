//! Billing cycle gRPC handlers.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::{BillingCycleStatus, ListBillingCyclesFilter, ListChargesFilter};
use crate::services::Database;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn get_billing_cycle(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetBillingCycleRequest>,
) -> Result<Response<GetBillingCycleResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_CYCLE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let cycle_id = parse_uuid(&req.cycle_id)?;

    tracing::debug!(tenant_id = %tenant_id, cycle_id = %cycle_id, "Getting billing cycle");

    let cycle = db
        .get_billing_cycle(tenant_id, cycle_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let cycle = cycle.ok_or_else(|| {
        Status::not_found("Billing cycle not found")
    })?;

    let charges = db
        .list_charges(tenant_id, cycle_id, &ListChargesFilter::default())
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    Ok(Response::new(GetBillingCycleResponse {
        billing_cycle: Some(cycle_to_proto(
            cycle,
            charges.into_iter().map(charge_to_proto).collect(),
        )),
    }))
}

pub async fn list_billing_cycles(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListBillingCyclesRequest>,
) -> Result<Response<ListBillingCyclesResponse>, Status> {
    let auth = capability_checker
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

    let cycles = db
        .list_billing_cycles(tenant_id, subscription_id, &filter)
        .await
        .map_err(|e| {
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

    Ok(Response::new(ListBillingCyclesResponse {
        billing_cycles: proto_cycles,
        next_page_token,
    }))
}

pub async fn advance_billing_cycle(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<AdvanceBillingCycleRequest>,
) -> Result<Response<AdvanceBillingCycleResponse>, Status> {
    let auth = capability_checker
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

    let (previous_cycle, new_cycle) = db
        .advance_billing_cycle(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to advance billing cycle");
            match e {
                service_core::error::AppError::NotFound(_) => Status::not_found(e.to_string()),
                service_core::error::AppError::BadRequest(_) => {
                    Status::failed_precondition(e.to_string())
                }
                _ => Status::internal(e.to_string()),
            }
        })?;

    Ok(Response::new(AdvanceBillingCycleResponse {
        previous_cycle: Some(cycle_to_proto(previous_cycle, vec![])),
        new_cycle: Some(cycle_to_proto(new_cycle, vec![])),
    }))
}
