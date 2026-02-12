//! gRPC handlers for transfer operations.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    check_feature_flag, check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::razorpay_transfers;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn create_transfer_from_payment(
    state: &AppState,
    request: Request<CreateTransferFromPaymentRequest>,
) -> Result<Response<CreateTransferFromPaymentResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_CREATE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
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

    if payment.status != models::TransactionStatus::Completed {
        return Err(Status::failed_precondition(
            "Payment must be in COMPLETED status for transfer",
        ));
    }

    // Validate linked account exists and is activated
    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.org_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    if account.status != models::LinkedAccountStatus::Activated {
        return Err(Status::failed_precondition(
            "Linked account must be ACTIVATED for transfer",
        ));
    }

    // Calculate commission
    let commission_amount = if let Some(ref commission) = account.commission {
        match commission.commission_type {
            models::CommissionType::Flat => commission.value,
            models::CommissionType::Percentage => req.amount * commission.value / 10000,
        }
    } else {
        0
    };

    // Validate provider_order_id exists for Razorpay API call
    let provider_payment_id = payment
        .provider_order_id
        .as_deref()
        .ok_or_else(|| Status::failed_precondition("Payment has no provider payment ID"))?;

    let rz_request = razorpay_transfers::CreatePaymentTransferRequest {
        transfers: vec![razorpay_transfers::TransferItem {
            account: account.razorpay_account_id.clone(),
            amount: req.amount,
            currency: req.currency.clone(),
            on_hold: Some(req.on_hold),
            on_hold_until: req.on_hold_until.map(|t| t.seconds as u64),
        }],
    };

    let rz_response = state
        .razorpay
        .create_payment_transfer(provider_payment_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay transfer");
            Status::internal(format!("Failed to create transfer: {}", e))
        })?;

    let rz_transfer = rz_response
        .items
        .into_iter()
        .next()
        .ok_or_else(|| Status::internal("No transfer returned from Razorpay"))?;

    let now = DateTime::now();
    let transfer = models::Transfer {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_transfer_id: rz_transfer.id,
        payment_id: Some(req.payment_id),
        order_id: None,
        linked_account_id: req.linked_account_id,
        amount: req.amount,
        currency: req.currency.clone(),
        status: models::TransferStatus::Created,
        on_hold: req.on_hold,
        on_hold_until: req
            .on_hold_until
            .map(|t| DateTime::from_millis(t.seconds * 1000)),
        commission_amount,
        amount_reversed: 0,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_transfer(transfer.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save transfer");
            Status::internal("Failed to save transfer")
        })?;

    Ok(Response::new(CreateTransferFromPaymentResponse {
        transfer: Some(transfer_to_proto(transfer)),
    }))
}

pub async fn create_transfer_from_order(
    state: &AppState,
    request: Request<CreateTransferFromOrderRequest>,
) -> Result<Response<CreateTransferFromOrderResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_CREATE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.org_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    if account.status != models::LinkedAccountStatus::Activated {
        return Err(Status::failed_precondition(
            "Linked account must be ACTIVATED",
        ));
    }

    let commission_amount = if let Some(ref commission) = account.commission {
        match commission.commission_type {
            models::CommissionType::Flat => commission.value,
            models::CommissionType::Percentage => req.amount * commission.value / 10000,
        }
    } else {
        0
    };

    let now = DateTime::now();
    let transfer = models::Transfer {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_transfer_id: format!("trf_pending_{}", Uuid::new_v4()),
        payment_id: None,
        order_id: Some(req.order_id),
        linked_account_id: req.linked_account_id,
        amount: req.amount,
        currency: req.currency.clone(),
        status: models::TransferStatus::Created,
        on_hold: req.on_hold,
        on_hold_until: None,
        commission_amount,
        amount_reversed: 0,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_transfer(transfer.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save transfer");
            Status::internal("Failed to save transfer")
        })?;

    Ok(Response::new(CreateTransferFromOrderResponse {
        transfer: Some(transfer_to_proto(transfer)),
    }))
}

pub async fn create_direct_transfer(
    state: &AppState,
    request: Request<CreateDirectTransferRequest>,
) -> Result<Response<CreateDirectTransferResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_CREATE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let account = state
        .repository
        .get_linked_account_in_tenant(&tenant.app_id, &tenant.org_id, &req.linked_account_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch linked account");
            Status::internal("Failed to fetch linked account")
        })?
        .ok_or_else(|| Status::not_found("Linked account not found"))?;

    if account.status != models::LinkedAccountStatus::Activated {
        return Err(Status::failed_precondition(
            "Linked account must be ACTIVATED",
        ));
    }

    let rz_request = razorpay_transfers::CreateDirectTransferRequest {
        account: account.razorpay_account_id.clone(),
        amount: req.amount,
        currency: req.currency.clone(),
    };

    let rz_response = state
        .razorpay
        .create_direct_transfer(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create direct transfer");
            Status::internal(format!("Failed to create transfer: {}", e))
        })?;

    let now = DateTime::now();
    let transfer = models::Transfer {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_transfer_id: rz_response.id,
        payment_id: None,
        order_id: None,
        linked_account_id: req.linked_account_id,
        amount: req.amount,
        currency: req.currency.clone(),
        status: models::TransferStatus::Created,
        on_hold: false,
        on_hold_until: None,
        commission_amount: 0,
        amount_reversed: 0,
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_transfer(transfer.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save transfer");
            Status::internal("Failed to save transfer")
        })?;

    Ok(Response::new(CreateDirectTransferResponse {
        transfer: Some(transfer_to_proto(transfer)),
    }))
}

pub async fn reverse_transfer(
    state: &AppState,
    request: Request<ReverseTransferRequest>,
) -> Result<Response<ReverseTransferResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_REVERSE)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let transfer = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    let reversal_amount = if req.amount == 0 {
        transfer.amount
    } else {
        req.amount
    };

    let rz_request = razorpay_transfers::ReverseTransferRequest {
        amount: reversal_amount,
    };
    state
        .razorpay
        .reverse_transfer(&transfer.razorpay_transfer_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to reverse transfer");
            Status::internal(format!("Failed to reverse transfer: {}", e))
        })?;

    let new_reversed = transfer.amount_reversed + reversal_amount;
    let new_status = if new_reversed >= transfer.amount {
        models::TransferStatus::Reversed
    } else {
        transfer.status.clone()
    };

    let update = mongodb::bson::doc! {
        "amount_reversed": new_reversed as i64,
        "status": mongodb::bson::to_bson(&new_status).map_err(|_| Status::internal("Serialization error"))?,
        "updated_at": DateTime::now()
    };
    state
        .repository
        .update_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id, update)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update transfer");
            Status::internal("Failed to update transfer")
        })?;

    let updated = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch updated transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    Ok(Response::new(ReverseTransferResponse {
        transfer: Some(transfer_to_proto(updated)),
    }))
}

pub async fn get_transfer(
    state: &AppState,
    request: Request<GetTransferRequest>,
) -> Result<Response<GetTransferResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let transfer = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    Ok(Response::new(GetTransferResponse {
        transfer: Some(transfer_to_proto(transfer)),
    }))
}

pub async fn list_transfers(
    state: &AppState,
    request: Request<ListTransfersRequest>,
) -> Result<Response<ListTransfersResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_transfer_status);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (transfers, total_count) = state
        .repository
        .list_transfers_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            req.linked_account_id.as_deref(),
            req.payment_id.as_deref(),
            status_filter,
            limit,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list transfers");
            Status::internal("Failed to list transfers")
        })?;

    Ok(Response::new(ListTransfersResponse {
        transfers: transfers.into_iter().map(transfer_to_proto).collect(),
        total_count,
    }))
}

pub async fn hold_transfer_settlement(
    state: &AppState,
    request: Request<HoldTransferSettlementRequest>,
) -> Result<Response<HoldTransferSettlementResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_HOLD)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let transfer = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    let rz_request = razorpay_transfers::UpdateTransferHoldRequest {
        on_hold: true,
        on_hold_until: req.on_hold_until.map(|t| t.seconds as u64),
    };
    state
        .razorpay
        .update_transfer_hold(&transfer.razorpay_transfer_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to hold transfer settlement");
            Status::internal(format!("Failed to hold transfer: {}", e))
        })?;

    let update = mongodb::bson::doc! {
        "on_hold": true,
        "updated_at": DateTime::now()
    };
    state
        .repository
        .update_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id, update)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update transfer");
            Status::internal("Failed to update transfer")
        })?;

    let updated = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    Ok(Response::new(HoldTransferSettlementResponse {
        transfer: Some(transfer_to_proto(updated)),
    }))
}

pub async fn release_transfer_settlement(
    state: &AppState,
    request: Request<ReleaseTransferSettlementRequest>,
) -> Result<Response<ReleaseTransferSettlementResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSFER_HOLD)
            .await?;
    }

    check_feature_flag(
        state.config.feature_flags.razorpay_route_enabled,
        "Razorpay Route",
    )?;
    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let transfer = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    let rz_request = razorpay_transfers::UpdateTransferHoldRequest {
        on_hold: false,
        on_hold_until: None,
    };
    state
        .razorpay
        .update_transfer_hold(&transfer.razorpay_transfer_id, rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to release transfer settlement");
            Status::internal(format!("Failed to release transfer: {}", e))
        })?;

    let update = mongodb::bson::doc! {
        "on_hold": false,
        "on_hold_until": mongodb::bson::Bson::Null,
        "updated_at": DateTime::now()
    };
    state
        .repository
        .update_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id, update)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update transfer");
            Status::internal("Failed to update transfer")
        })?;

    let updated = state
        .repository
        .get_transfer_in_tenant(&tenant.app_id, &tenant.org_id, &req.transfer_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch transfer");
            Status::internal("Failed to fetch transfer")
        })?
        .ok_or_else(|| Status::not_found("Transfer not found"))?;

    Ok(Response::new(ReleaseTransferSettlementResponse {
        transfer: Some(transfer_to_proto(updated)),
    }))
}

fn transfer_to_proto(t: models::Transfer) -> Transfer {
    Transfer {
        id: t.id,
        app_id: t.app_id,
        org_id: t.org_id,
        razorpay_transfer_id: t.razorpay_transfer_id,
        payment_id: t.payment_id,
        order_id: t.order_id,
        linked_account_id: t.linked_account_id,
        amount: t.amount,
        currency: t.currency,
        status: transfer_status_to_proto(&t.status).into(),
        on_hold: t.on_hold,
        on_hold_until: t.on_hold_until.and_then(datetime_to_timestamp),
        commission_amount: t.commission_amount,
        amount_reversed: t.amount_reversed,
        created_at: datetime_to_timestamp(t.created_at),
        updated_at: datetime_to_timestamp(t.updated_at),
    }
}

fn transfer_status_to_proto(status: &models::TransferStatus) -> TransferStatus {
    match status {
        models::TransferStatus::Created => TransferStatus::Created,
        models::TransferStatus::Pending => TransferStatus::Pending,
        models::TransferStatus::Processed => TransferStatus::Processed,
        models::TransferStatus::Reversed => TransferStatus::Reversed,
        models::TransferStatus::Failed => TransferStatus::Failed,
    }
}

fn proto_to_transfer_status(status: i32) -> Option<models::TransferStatus> {
    match TransferStatus::try_from(status) {
        Ok(TransferStatus::Created) => Some(models::TransferStatus::Created),
        Ok(TransferStatus::Pending) => Some(models::TransferStatus::Pending),
        Ok(TransferStatus::Processed) => Some(models::TransferStatus::Processed),
        Ok(TransferStatus::Reversed) => Some(models::TransferStatus::Reversed),
        Ok(TransferStatus::Failed) => Some(models::TransferStatus::Failed),
        _ => None,
    }
}
