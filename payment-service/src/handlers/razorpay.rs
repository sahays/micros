//! Razorpay payment handlers.
//!
//! Implements order creation, payment verification, and webhook handling
//! for Razorpay payment integration.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use service_core::error::AppError;
use uuid::Uuid;

use crate::{
    middleware::TenantContext,
    models::{Transaction, TransactionStatus},
    services::razorpay::PaymentVerification,
    AppState,
};

/// Request to create a new Razorpay order.
#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    /// Amount in the smallest currency unit (e.g., paise for INR).
    pub amount: u64,
    /// Currency code (e.g., "INR").
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Optional receipt ID for tracking.
    pub receipt: Option<String>,
    /// Optional notes to attach to the order.
    pub notes: Option<serde_json::Value>,
}

fn default_currency() -> String {
    "INR".to_string()
}

/// Response after creating a Razorpay order.
#[derive(Debug, Serialize)]
pub struct CreateOrderResponse {
    /// Internal transaction ID.
    pub transaction_id: Uuid,
    /// Razorpay order ID (use this in frontend checkout).
    pub razorpay_order_id: String,
    /// Amount in smallest currency unit.
    pub amount: u64,
    /// Currency code.
    pub currency: String,
    /// Razorpay key ID (for frontend initialization).
    pub razorpay_key_id: String,
}

/// Request to verify a payment after checkout.
#[derive(Debug, Deserialize)]
pub struct VerifyPaymentRequest {
    /// Internal transaction ID.
    pub transaction_id: Uuid,
    /// Razorpay order ID.
    pub razorpay_order_id: String,
    /// Razorpay payment ID.
    pub razorpay_payment_id: String,
    /// Razorpay signature for verification.
    pub razorpay_signature: String,
}

/// Response after verifying a payment.
#[derive(Debug, Serialize)]
pub struct VerifyPaymentResponse {
    pub transaction_id: Uuid,
    pub status: TransactionStatus,
    pub razorpay_payment_id: String,
    pub message: String,
}

/// Create a new Razorpay order.
///
/// This creates both a local transaction record and a Razorpay order.
/// The client should use the returned `razorpay_order_id` to initiate checkout.
pub async fn create_order(
    State(state): State<AppState>,
    tenant: TenantContext,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<CreateOrderResponse>), AppError> {
    tracing::info!(
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        amount = payload.amount,
        currency = %payload.currency,
        "Creating Razorpay order"
    );

    // Check if Razorpay is configured
    if !state.razorpay.is_configured() {
        return Err(AppError::InternalError(anyhow::anyhow!(
            "Razorpay is not configured for this environment"
        )));
    }

    // Create Razorpay order
    let razorpay_order = state
        .razorpay
        .create_order(
            payload.amount,
            &payload.currency,
            payload.receipt.clone(),
            payload.notes.clone(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Razorpay order");
            AppError::InternalError(anyhow::anyhow!("Failed to create payment order: {}", e))
        })?;

    // Create local transaction record
    let now = DateTime::now();
    let transaction = Transaction {
        id: Uuid::new_v4(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        user_id: tenant.user_id.clone(),
        amount: payload.amount as f64 / 100.0, // Convert from paise to rupees
        currency: payload.currency.clone(),
        status: TransactionStatus::Created,
        provider_order_id: Some(razorpay_order.id.clone()),
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_transaction(transaction.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save transaction");
            AppError::InternalError(anyhow::anyhow!("Failed to save transaction"))
        })?;

    tracing::info!(
        transaction_id = %transaction.id,
        razorpay_order_id = %razorpay_order.id,
        "Razorpay order created successfully"
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateOrderResponse {
            transaction_id: transaction.id,
            razorpay_order_id: razorpay_order.id,
            amount: payload.amount,
            currency: payload.currency,
            razorpay_key_id: state.config.razorpay.key_id.clone(),
        }),
    ))
}

/// Verify payment after Razorpay checkout completion.
///
/// This verifies the Razorpay signature and updates the transaction status.
pub async fn verify_payment(
    State(state): State<AppState>,
    tenant: TenantContext,
    Json(payload): Json<VerifyPaymentRequest>,
) -> Result<Json<VerifyPaymentResponse>, AppError> {
    tracing::info!(
        transaction_id = %payload.transaction_id,
        razorpay_order_id = %payload.razorpay_order_id,
        razorpay_payment_id = %payload.razorpay_payment_id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        "Verifying Razorpay payment"
    );

    // Fetch transaction within tenant scope
    let transaction = state
        .repository
        .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, payload.transaction_id)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Transaction not found")))?;

    // Verify the order ID matches
    if transaction.provider_order_id.as_deref() != Some(&payload.razorpay_order_id) {
        tracing::warn!(
            transaction_id = %payload.transaction_id,
            expected_order_id = ?transaction.provider_order_id,
            received_order_id = %payload.razorpay_order_id,
            "Order ID mismatch"
        );
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Order ID does not match transaction"
        )));
    }

    // Verify the signature
    let verification = PaymentVerification {
        razorpay_order_id: payload.razorpay_order_id.clone(),
        razorpay_payment_id: payload.razorpay_payment_id.clone(),
        razorpay_signature: payload.razorpay_signature.clone(),
    };

    let is_valid = state
        .razorpay
        .verify_payment_signature(&verification)
        .map_err(|e| {
            tracing::error!(error = %e, "Signature verification error");
            AppError::InternalError(anyhow::anyhow!("Signature verification failed"))
        })?;

    let (new_status, message) = if is_valid {
        (
            TransactionStatus::Completed,
            "Payment verified successfully",
        )
    } else {
        (
            TransactionStatus::Failed,
            "Payment verification failed - invalid signature",
        )
    };

    // Update transaction status
    state
        .repository
        .update_transaction_status_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            payload.transaction_id,
            new_status.clone(),
        )
        .await?;

    tracing::info!(
        transaction_id = %payload.transaction_id,
        status = ?new_status,
        "Payment verification completed"
    );

    Ok(Json(VerifyPaymentResponse {
        transaction_id: payload.transaction_id,
        status: new_status,
        razorpay_payment_id: payload.razorpay_payment_id,
        message: message.to_string(),
    }))
}

/// Razorpay webhook handler.
///
/// Receives and processes webhook events from Razorpay.
/// Verifies the webhook signature before processing.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, AppError> {
    // Extract signature from headers
    let signature = headers
        .get("X-Razorpay-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("Missing X-Razorpay-Signature header");
            AppError::Unauthorized(anyhow::anyhow!("Missing webhook signature"))
        })?;

    tracing::debug!(signature = %signature, "Received Razorpay webhook");

    // Verify webhook signature
    let is_valid = state
        .razorpay
        .verify_webhook_signature(&body, signature)
        .map_err(|e| {
            tracing::error!(error = %e, "Webhook signature verification error");
            AppError::InternalError(anyhow::anyhow!("Webhook verification failed"))
        })?;

    if !is_valid {
        tracing::warn!("Invalid webhook signature");
        return Err(AppError::Unauthorized(anyhow::anyhow!(
            "Invalid webhook signature"
        )));
    }

    // Parse the webhook event
    let event = state.razorpay.parse_webhook_event(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse webhook event");
        AppError::BadRequest(anyhow::anyhow!("Invalid webhook payload"))
    })?;

    tracing::info!(
        event_type = %event.event,
        account_id = %event.account_id,
        "Processing Razorpay webhook"
    );

    // Handle different event types
    match event.event.as_str() {
        "payment.captured" => {
            if let Some(ref payment_entity) = event.payload.payment {
                let payment = &payment_entity.entity;
                tracing::info!(
                    payment_id = %payment.id,
                    order_id = ?payment.order_id,
                    amount = payment.amount,
                    status = %payment.status,
                    "Payment captured webhook received"
                );

                // Update transaction status if we have an order_id
                if let Some(ref order_id) = payment.order_id {
                    if let Err(e) = update_transaction_by_order_id(
                        &state,
                        order_id,
                        TransactionStatus::Completed,
                    )
                    .await
                    {
                        tracing::error!(
                            order_id = %order_id,
                            error = %e,
                            "Failed to update transaction from webhook"
                        );
                    }
                }
            }
        }
        "payment.failed" => {
            if let Some(ref payment_entity) = event.payload.payment {
                let payment = &payment_entity.entity;
                tracing::info!(
                    payment_id = %payment.id,
                    order_id = ?payment.order_id,
                    "Payment failed webhook received"
                );

                if let Some(ref order_id) = payment.order_id {
                    if let Err(e) =
                        update_transaction_by_order_id(&state, order_id, TransactionStatus::Failed)
                            .await
                    {
                        tracing::error!(
                            order_id = %order_id,
                            error = %e,
                            "Failed to update transaction from webhook"
                        );
                    }
                }
            }
        }
        "order.paid" => {
            if let Some(ref order_entity) = event.payload.order {
                let order = &order_entity.entity;
                tracing::info!(
                    order_id = %order.id,
                    amount = order.amount,
                    "Order paid webhook received"
                );

                if let Err(e) =
                    update_transaction_by_order_id(&state, &order.id, TransactionStatus::Completed)
                        .await
                {
                    tracing::error!(
                        order_id = %order.id,
                        error = %e,
                        "Failed to update transaction from webhook"
                    );
                }
            }
        }
        "refund.created" | "refund.processed" => {
            tracing::info!(event_type = %event.event, "Refund webhook received");
            // Handle refund events - would update transaction to Refunded status
        }
        _ => {
            tracing::debug!(event_type = %event.event, "Unhandled webhook event type");
        }
    }

    // Always return 200 OK to acknowledge receipt
    Ok(StatusCode::OK)
}

/// Helper to update transaction by Razorpay order ID.
async fn update_transaction_by_order_id(
    state: &AppState,
    order_id: &str,
    status: TransactionStatus,
) -> anyhow::Result<()> {
    // Find transaction by provider_order_id
    // Note: In a production system, you'd have a dedicated method for this
    // For now, we'll use the existing get_transaction method with a filter
    use mongodb::bson::doc;

    let filter = doc! { "provider_order_id": order_id };
    let update = doc! {
        "$set": {
            "status": mongodb::bson::to_bson(&status)?,
            "updated_at": mongodb::bson::DateTime::now()
        }
    };

    state
        .db
        .collection::<Transaction>("transactions")
        .update_one(filter, update, None)
        .await?;

    tracing::info!(
        order_id = %order_id,
        status = ?status,
        "Transaction updated via webhook"
    );

    Ok(())
}

/// Get a transaction by ID (for status checking).
pub async fn get_transaction(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<TransactionResponse>, AppError> {
    let transaction = state
        .repository
        .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, transaction_id)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Transaction not found")))?;

    Ok(Json(TransactionResponse::from(transaction)))
}

/// Transaction response DTO.
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub status: TransactionStatus,
    pub provider_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Transaction> for TransactionResponse {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            amount: t.amount,
            currency: t.currency,
            status: t.status,
            provider_order_id: t.provider_order_id,
            created_at: t.created_at.to_string(),
            updated_at: t.updated_at.to_string(),
        }
    }
}
