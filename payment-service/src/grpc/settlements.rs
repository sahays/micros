//! gRPC handlers for settlement operations.

use crate::grpc::helpers::{
    check_feature_flag, check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_settlements;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn request_on_demand_settlement(
    state: &AppState,
    request: Request<RequestOnDemandSettlementRequest>,
) -> Result<Response<RequestOnDemandSettlementResponse>, Status> {
    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let rz_request = razorpay_settlements::CreateOnDemandSettlementRequest {
        amount: req.amount,
        settle_full_balance: if req.amount == 0 { Some(true) } else { None },
    };

    let rz_response = state
        .razorpay
        .create_on_demand_settlement(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create on-demand settlement");
            Status::internal(format!("Failed to create settlement: {}", e))
        })?;

    let now = DateTime::now();
    let settlement = models::Settlement {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        tenant_id: tenant.tenant_id.clone(),
        razorpay_settlement_id: rz_response.id,
        linked_account_id: req.linked_account_id,
        amount: rz_response.amount,
        currency: req.currency,
        status: models::SettlementStatus::Created,
        settlement_type: models::SettlementType::OnDemand,
        utr: rz_response.utr,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_settlement(settlement.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save settlement");
            Status::internal("Failed to save settlement")
        })?;

    Ok(Response::new(RequestOnDemandSettlementResponse {
        settlement: Some(settlement_to_proto(settlement)),
    }))
}

pub async fn get_settlement(
    state: &AppState,
    request: Request<GetSettlementRequest>,
) -> Result<Response<GetSettlementResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let settlement = state
        .repository
        .get_settlement_in_tenant(&tenant.app_id, &tenant.tenant_id, &req.settlement_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch settlement");
            Status::internal("Failed to fetch settlement")
        })?
        .ok_or_else(|| Status::not_found("Settlement not found"))?;

    Ok(Response::new(GetSettlementResponse {
        settlement: Some(settlement_to_proto(settlement)),
    }))
}

pub async fn list_settlements(
    state: &AppState,
    request: Request<ListSettlementsRequest>,
) -> Result<Response<ListSettlementsResponse>, Status> {
    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_settlement_status);
    let type_filter = req.settlement_type.and_then(proto_to_settlement_type);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (settlements, total_count) = state
        .repository
        .list_settlements_in_tenant(
            &tenant.app_id,
            &tenant.tenant_id,
            req.linked_account_id.as_deref(),
            status_filter,
            type_filter,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list settlements");
            Status::internal("Failed to list settlements")
        })?;

    Ok(Response::new(ListSettlementsResponse {
        settlements: settlements.into_iter().map(settlement_to_proto).collect(),
        total_count,
    }))
}

fn settlement_to_proto(s: models::Settlement) -> Settlement {
    Settlement {
        id: s.id,
        app_id: s.app_id,
        tenant_id: s.tenant_id,
        razorpay_settlement_id: s.razorpay_settlement_id,
        linked_account_id: s.linked_account_id,
        amount: s.amount,
        currency: s.currency,
        status: settlement_status_to_proto(&s.status).into(),
        settlement_type: settlement_type_to_proto(&s.settlement_type).into(),
        utr: s.utr,
        created_at: datetime_to_timestamp(s.created_at),
        updated_at: datetime_to_timestamp(s.updated_at),
    }
}

fn settlement_status_to_proto(status: &models::SettlementStatus) -> SettlementStatus {
    match status {
        models::SettlementStatus::Created => SettlementStatus::Created,
        models::SettlementStatus::Processed => SettlementStatus::Processed,
        models::SettlementStatus::Failed => SettlementStatus::Failed,
    }
}

fn settlement_type_to_proto(t: &models::SettlementType) -> SettlementType {
    match t {
        models::SettlementType::Normal => SettlementType::Normal,
        models::SettlementType::OnDemand => SettlementType::OnDemand,
    }
}

fn proto_to_settlement_status(status: i32) -> Option<models::SettlementStatus> {
    match SettlementStatus::try_from(status) {
        Ok(SettlementStatus::Created) => Some(models::SettlementStatus::Created),
        Ok(SettlementStatus::Processed) => Some(models::SettlementStatus::Processed),
        Ok(SettlementStatus::Failed) => Some(models::SettlementStatus::Failed),
        _ => None,
    }
}

fn proto_to_settlement_type(t: i32) -> Option<models::SettlementType> {
    match SettlementType::try_from(t) {
        Ok(SettlementType::Normal) => Some(models::SettlementType::Normal),
        Ok(SettlementType::OnDemand) => Some(models::SettlementType::OnDemand),
        _ => None,
    }
}
