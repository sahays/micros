//! gRPC handlers for direct UPI and offline payment recording.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{extract_tenant_context, transaction_to_proto};
use crate::grpc::proto::*;
use crate::models::{PaymentChannel, PaymentMethodType, Transaction, TransactionStatus};
use crate::services::metrics::{record_amount, record_transaction};
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// Record a payment received directly via UPI (bypassing payment gateway).
pub async fn record_direct_upi_payment(
    state: &AppState,
    request: Request<RecordDirectUpiPaymentRequest>,
) -> Result<Response<RecordDirectUpiPaymentResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_DIRECT_UPI_RECORD)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Validate amount
    if req.amount_paise == 0 {
        return Err(Status::invalid_argument("Amount must be greater than 0"));
    }

    // Validate UTR
    let utr = req.utr.trim().to_string();
    if utr.is_empty() {
        return Err(Status::invalid_argument("UTR must not be empty"));
    }

    // Check for duplicate UTR within tenant
    let existing = state
        .repository
        .get_transaction_by_external_ref(&tenant.app_id, &tenant.org_id, &utr)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check for duplicate UTR");
            Status::internal("Failed to check for duplicate UTR")
        })?;

    if existing.is_some() {
        return Err(Status::already_exists(format!(
            "A transaction with UTR '{}' already exists",
            utr
        )));
    }

    let now = DateTime::now();
    let transaction_id = Uuid::new_v4().to_string();
    let transaction = Transaction {
        id: transaction_id.clone(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        user_id: tenant.user_id.clone(),
        amount_paise: req.amount_paise,
        currency: req.currency.clone(),
        status: TransactionStatus::Completed,
        provider_order_id: None,
        linked_account_id: None,
        subscription_id: None,
        payment_link_id: None,
        payment_channel: Some(PaymentChannel::DirectUpi),
        payment_method_type: Some(PaymentMethodType::Upi),
        external_reference: Some(utr),
        notes: req.notes,
        created_at: now,
        updated_at: now,
    };

    tracing::info!(
        transaction_id = %transaction_id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        amount_paise = req.amount_paise,
        "Recording direct UPI payment"
    );

    state
        .repository
        .create_transaction(transaction.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create direct UPI transaction");
            Status::internal("Failed to record direct UPI payment")
        })?;

    record_transaction(&tenant.app_id, "completed");
    record_amount(&tenant.app_id, &req.currency, req.amount_paise);

    Ok(Response::new(RecordDirectUpiPaymentResponse {
        transaction: Some(transaction_to_proto(transaction)),
    }))
}

/// Record an offline payment (cash, cheque, bank transfer, etc.).
pub async fn record_offline_payment(
    state: &AppState,
    request: Request<RecordOfflinePaymentRequest>,
) -> Result<Response<RecordOfflinePaymentResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_OFFLINE_RECORD)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    // Validate amount
    if req.amount_paise == 0 {
        return Err(Status::invalid_argument("Amount must be greater than 0"));
    }

    // Convert and validate payment method type
    let method_type = match crate::grpc::helpers::proto_to_payment_method_type(
        req.payment_method_type,
    ) {
        Some(
            m @ (PaymentMethodType::Cash
            | PaymentMethodType::Cheque
            | PaymentMethodType::BankTransfer
            | PaymentMethodType::Other),
        ) => m,
        Some(_) => {
            return Err(Status::invalid_argument(
                "Invalid payment method type for offline payment. Must be CASH, CHEQUE, BANK_TRANSFER, or OTHER",
            ));
        }
        None => {
            return Err(Status::invalid_argument("Payment method type is required"));
        }
    };

    // Check for duplicate external reference if provided
    let external_reference = req
        .external_reference
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());

    if let Some(ref ext_ref) = external_reference {
        let existing = state
            .repository
            .get_transaction_by_external_ref(&tenant.app_id, &tenant.org_id, ext_ref)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check for duplicate external reference");
                Status::internal("Failed to check for duplicate reference")
            })?;

        if existing.is_some() {
            return Err(Status::already_exists(format!(
                "A transaction with external reference '{}' already exists",
                ext_ref
            )));
        }
    }

    let now = DateTime::now();
    let transaction_id = Uuid::new_v4().to_string();
    let transaction = Transaction {
        id: transaction_id.clone(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        user_id: tenant.user_id.clone(),
        amount_paise: req.amount_paise,
        currency: req.currency.clone(),
        status: TransactionStatus::Completed,
        provider_order_id: None,
        linked_account_id: None,
        subscription_id: None,
        payment_link_id: None,
        payment_channel: Some(PaymentChannel::Offline),
        payment_method_type: Some(method_type),
        external_reference,
        notes: req.notes,
        created_at: now,
        updated_at: now,
    };

    tracing::info!(
        transaction_id = %transaction_id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        amount_paise = req.amount_paise,
        payment_method_type = ?transaction.payment_method_type,
        "Recording offline payment"
    );

    state
        .repository
        .create_transaction(transaction.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create offline transaction");
            Status::internal("Failed to record offline payment")
        })?;

    record_transaction(&tenant.app_id, "completed");
    record_amount(&tenant.app_id, &req.currency, req.amount_paise);

    Ok(Response::new(RecordOfflinePaymentResponse {
        transaction: Some(transaction_to_proto(transaction)),
    }))
}
