//! gRPC handlers for refund operations.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_refunds;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn initiate_refund(
    state: &AppState,
    request: Request<InitiateRefundRequest>,
) -> Result<Response<InitiateRefundResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_REFUND_CREATE)
            .await?;
    }

    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Validate payment exists and is completed
    let payment = state
        .repository
        .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, &req.payment_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch payment");
            Status::internal("Failed to fetch payment")
        })?
        .ok_or_else(|| Status::not_found("Payment not found"))?;

    if payment.status != models::TransactionStatus::Completed
        && payment.status != models::TransactionStatus::PartiallyRefunded
    {
        return Err(Status::failed_precondition(
            "Payment must be COMPLETED or PARTIALLY_REFUNDED",
        ));
    }

    // Calculate available refund amount
    let payment_amount_paise = payment.amount_paise;
    let prior_refunds = state
        .repository
        .sum_refunds_for_payment(&tenant.app_id, &tenant.org_id, &req.payment_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to sum prior refunds");
            Status::internal("Failed to calculate refund availability")
        })?;

    let available = payment_amount_paise.saturating_sub(prior_refunds);
    let refund_amount = if req.amount == 0 {
        available
    } else {
        req.amount
    };

    if refund_amount > available {
        return Err(Status::invalid_argument(format!(
            "Refund amount {} exceeds available amount {}",
            refund_amount, available
        )));
    }

    let speed_str = match RefundSpeed::try_from(req.speed) {
        Ok(RefundSpeed::Optimum) => Some("optimum".to_string()),
        Ok(RefundSpeed::Normal) => Some("normal".to_string()),
        _ => None,
    };

    let provider_payment_id = payment
        .provider_order_id
        .as_deref()
        .ok_or_else(|| Status::failed_precondition("Payment has no provider payment ID"))?;

    let rz_request = razorpay_refunds::CreateRefundRequest {
        amount: refund_amount,
        speed: speed_str,
        notes: req
            .reason
            .as_ref()
            .map(|r| serde_json::json!({"reason": r})),
        reverse_all: if req.reverse_all_transfers {
            Some(true)
        } else {
            None
        },
    };

    let rz_response = state
        .razorpay
        .create_refund(provider_payment_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay refund");
            Status::internal(format!("Failed to create refund: {}", e))
        })?;

    let speed = match req.speed {
        x if x == RefundSpeed::Optimum as i32 => models::RefundSpeed::Optimum,
        _ => models::RefundSpeed::Normal,
    };

    let now = DateTime::now();
    let refund = models::Refund {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_refund_id: rz_response.id,
        payment_id: req.payment_id.clone(),
        amount: refund_amount,
        currency: payment.currency.clone(),
        status: models::RefundStatus::Created,
        speed: speed.clone(),
        reason: req.reason,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_refund(refund.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save refund");
            Status::internal("Failed to save refund")
        })?;

    // Update transaction status
    let new_tx_status = if prior_refunds + refund_amount >= payment_amount_paise {
        models::TransactionStatus::Refunded
    } else {
        models::TransactionStatus::PartiallyRefunded
    };

    state
        .repository
        .update_transaction_status_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            &req.payment_id,
            new_tx_status,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update transaction status");
            Status::internal("Failed to update transaction status")
        })?;

    Ok(Response::new(InitiateRefundResponse {
        refund: Some(refund_to_proto(refund)),
    }))
}

pub async fn get_refund(
    state: &AppState,
    request: Request<GetRefundRequest>,
) -> Result<Response<GetRefundResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_REFUND_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let refund = state
        .repository
        .get_refund_in_tenant(&tenant.app_id, &tenant.org_id, &req.refund_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch refund");
            Status::internal("Failed to fetch refund")
        })?
        .ok_or_else(|| Status::not_found("Refund not found"))?;

    Ok(Response::new(GetRefundResponse {
        refund: Some(refund_to_proto(refund)),
    }))
}

pub async fn list_refunds(
    state: &AppState,
    request: Request<ListRefundsRequest>,
) -> Result<Response<ListRefundsResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_REFUND_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_refund_status);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (refunds, total_count) = state
        .repository
        .list_refunds_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            req.payment_id.as_deref(),
            status_filter,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list refunds");
            Status::internal("Failed to list refunds")
        })?;

    Ok(Response::new(ListRefundsResponse {
        refunds: refunds.into_iter().map(refund_to_proto).collect(),
        total_count,
    }))
}

fn refund_to_proto(r: models::Refund) -> Refund {
    Refund {
        id: r.id,
        app_id: r.app_id,
        org_id: r.org_id,
        razorpay_refund_id: r.razorpay_refund_id,
        payment_id: r.payment_id,
        amount: r.amount,
        currency: r.currency,
        status: refund_status_to_proto(&r.status).into(),
        speed: refund_speed_to_proto(&r.speed).into(),
        reason: r.reason,
        created_at: datetime_to_timestamp(r.created_at),
        updated_at: datetime_to_timestamp(r.updated_at),
    }
}

fn refund_status_to_proto(s: &models::RefundStatus) -> RefundStatus {
    match s {
        models::RefundStatus::Created => RefundStatus::Created,
        models::RefundStatus::Processed => RefundStatus::Processed,
        models::RefundStatus::Failed => RefundStatus::Failed,
    }
}

fn refund_speed_to_proto(s: &models::RefundSpeed) -> RefundSpeed {
    match s {
        models::RefundSpeed::Normal => RefundSpeed::Normal,
        models::RefundSpeed::Optimum => RefundSpeed::Optimum,
    }
}

fn proto_to_refund_status(s: i32) -> Option<models::RefundStatus> {
    match RefundStatus::try_from(s) {
        Ok(RefundStatus::Created) => Some(models::RefundStatus::Created),
        Ok(RefundStatus::Processed) => Some(models::RefundStatus::Processed),
        Ok(RefundStatus::Failed) => Some(models::RefundStatus::Failed),
        _ => None,
    }
}
