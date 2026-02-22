//! Charge gRPC handlers.

use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::{ChargeType, CreateCharge, ListChargesFilter, SubscriptionStatus};
use crate::services::Database;
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn get_charge(
    db: &Arc<Database>,
    request: Request<GetChargeRequest>,
) -> Result<Response<GetChargeResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    let charge_id = parse_uuid(&req.charge_id)?;

    tracing::debug!(tenant_id = %tenant_id, charge_id = %charge_id, "Getting charge");

    let charge = db.get_charge(tenant_id, charge_id).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    let charge = charge.ok_or_else(|| {
        Status::not_found("Charge not found")
    })?;

    Ok(Response::new(GetChargeResponse {
        charge: Some(charge_to_proto(charge)),
    }))
}

pub async fn list_charges(
    db: &Arc<Database>,
    request: Request<ListChargesRequest>,
) -> Result<Response<ListChargesResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

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

    let charges = db
        .list_charges(tenant_id, cycle_id, &filter)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let proto_charges: Vec<_> = charges.into_iter().map(charge_to_proto).collect();
    let next_page_token = proto_charges
        .last()
        .map(|c| c.charge_id.clone())
        .unwrap_or_default();

    Ok(Response::new(ListChargesResponse {
        charges: proto_charges,
        next_page_token,
    }))
}

pub async fn create_one_time_charge(
    db: &Arc<Database>,
    request: Request<CreateOneTimeChargeRequest>,
) -> Result<Response<CreateOneTimeChargeResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::info!(
        tenant_id = %tenant_id,
        subscription_id = %subscription_id,
        amount = %req.amount,
        "Creating one-time charge"
    );

    // Validate subscription exists and is active
    let subscription = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| Status::not_found("Subscription not found"))?;

    if subscription.status != SubscriptionStatus::Active.as_str() {
        return Err(Status::failed_precondition(
            "Subscription must be active to add charges",
        ));
    }

    // Get current billing cycle
    let cycle = db
        .get_current_billing_cycle(subscription_id)
        .await
        .map_err(|e| {
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

    let charge = db.create_charge(&input).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create charge");
        Status::internal(e.to_string())
    })?;

    Ok(Response::new(CreateOneTimeChargeResponse {
        charge: Some(charge_to_proto(charge)),
    }))
}
