//! Webhook event processing for all Razorpay event types.

use crate::models;
use crate::services::metrics::{
    record_linked_account, record_subscription, record_subscription_charge, record_webhook_event,
};
use crate::services::razorpay::WebhookEvent;
use crate::startup::AppState;
use tonic::Status;

/// Process a Razorpay webhook event and return the event type string.
pub async fn process_webhook_event(
    state: &AppState,
    event: &WebhookEvent,
) -> Result<String, Status> {
    let event_type = event.event.as_str();

    // Determine domain for metrics
    let domain = event_type.split('.').next().unwrap_or("unknown");
    record_webhook_event(event_type, domain);

    match event_type {
        // --- Payment events ---
        "payment.authorized" | "payment.captured" => {
            if let Some(ref payment_entity) = event.payload.payment {
                let payment = &payment_entity.entity;
                tracing::info!(
                    payment_id = %payment.id,
                    order_id = ?payment.order_id,
                    amount = payment.amount,
                    "Payment {} webhook received", event_type
                );

                if let Some(ref order_id) = payment.order_id {
                    update_transaction_by_order_id(
                        state,
                        order_id,
                        models::TransactionStatus::Completed,
                    )
                    .await;
                }
            }
        }
        "payment.failed" => {
            if let Some(ref payment_entity) = event.payload.payment {
                let payment = &payment_entity.entity;
                tracing::info!(payment_id = %payment.id, "Payment failed webhook received");

                if let Some(ref order_id) = payment.order_id {
                    update_transaction_by_order_id(
                        state,
                        order_id,
                        models::TransactionStatus::Failed,
                    )
                    .await;
                }
            }
        }

        // --- Order events ---
        "order.paid" => {
            if let Some(ref order_entity) = event.payload.order {
                let order = &order_entity.entity;
                tracing::info!(order_id = %order.id, "Order paid webhook received");
                update_transaction_by_order_id(
                    state,
                    &order.id,
                    models::TransactionStatus::Completed,
                )
                .await;
            }
        }

        // --- Transfer events ---
        "transfer.processed" => {
            if let Some(ref transfer_entity) = event.payload.transfer {
                let transfer = &transfer_entity.entity;
                tracing::info!(transfer_id = %transfer.id, "Transfer processed webhook received");

                if let Err(e) = state
                    .repository
                    .update_transfer_status_by_razorpay_id(
                        &transfer.id,
                        models::TransferStatus::Processed,
                    )
                    .await
                {
                    tracing::error!(error = %e, "Failed to update transfer status");
                }
            }
        }
        "transfer.reversed" => {
            if let Some(ref transfer_entity) = event.payload.transfer {
                let transfer = &transfer_entity.entity;
                tracing::info!(transfer_id = %transfer.id, "Transfer reversed webhook received");

                if let Err(e) = state
                    .repository
                    .update_transfer_status_by_razorpay_id(
                        &transfer.id,
                        models::TransferStatus::Reversed,
                    )
                    .await
                {
                    tracing::error!(error = %e, "Failed to update transfer status");
                }
            }
        }
        "transfer.failed" => {
            if let Some(ref transfer_entity) = event.payload.transfer {
                let transfer = &transfer_entity.entity;
                tracing::info!(transfer_id = %transfer.id, "Transfer failed webhook received");

                if let Err(e) = state
                    .repository
                    .update_transfer_status_by_razorpay_id(
                        &transfer.id,
                        models::TransferStatus::Failed,
                    )
                    .await
                {
                    tracing::error!(error = %e, "Failed to update transfer status");
                }
            }
        }

        // --- Settlement events ---
        "settlement.processed" => {
            if let Some(ref settlement_entity) = event.payload.settlement {
                let settlement = &settlement_entity.entity;
                tracing::info!(settlement_id = %settlement.id, "Settlement processed webhook received");

                let update = mongodb::bson::doc! {
                    "status": mongodb::bson::to_bson(&models::SettlementStatus::Processed).unwrap_or_default(),
                    "utr": settlement.utr.as_deref().unwrap_or(""),
                    "updated_at": mongodb::bson::DateTime::now()
                };
                if let Err(e) = state
                    .repository
                    .update_settlement_by_razorpay_id(&settlement.id, update)
                    .await
                {
                    tracing::error!(error = %e, "Failed to update settlement status");
                }
            }
        }

        // --- Subscription events ---
        "subscription.authenticated" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Authenticated)
                .await;
        }
        "subscription.active" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Active).await;
            if let Some(ref sub_entity) = event.payload.subscription {
                record_subscription(&sub_entity.entity.id, "active");
            }
        }
        "subscription.pending" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Pending).await;
        }
        "subscription.halted" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Halted).await;
        }
        "subscription.paused" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Paused).await;
        }
        "subscription.resumed" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Active).await;
        }
        "subscription.cancelled" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Cancelled).await;
        }
        "subscription.completed" => {
            update_subscription_status(state, event, models::SubscriptionStatus::Completed).await;
        }
        "subscription.charged" => {
            if let Some(ref sub_entity) = event.payload.subscription {
                let sub = &sub_entity.entity;
                tracing::info!(subscription_id = %sub.id, "Subscription charged webhook received");

                if let Err(e) = state
                    .repository
                    .increment_subscription_charge_count(&sub.id)
                    .await
                {
                    tracing::error!(error = %e, "Failed to increment subscription charge count");
                }
                record_subscription_charge(&sub.id, "charged");
            }
        }

        // --- Account events ---
        "account.under_review" => {
            update_account_status(state, event, models::LinkedAccountStatus::UnderReview).await;
        }
        "account.needs_clarification" => {
            update_account_status(
                state,
                event,
                models::LinkedAccountStatus::NeedsClarification,
            )
            .await;
        }
        "account.activated" => {
            update_account_status(state, event, models::LinkedAccountStatus::Activated).await;
            if let Some(ref account_entity) = event.payload.account {
                record_linked_account(&account_entity.entity.id, "activated");
            }
        }
        "account.suspended" => {
            update_account_status(state, event, models::LinkedAccountStatus::Suspended).await;
        }

        // --- Payment link events ---
        "payment_link.paid" => {
            update_payment_link_status(state, event, models::PaymentLinkStatus::Paid).await;
        }
        "payment_link.partially_paid" => {
            update_payment_link_status(state, event, models::PaymentLinkStatus::PartiallyPaid)
                .await;
        }
        "payment_link.cancelled" => {
            update_payment_link_status(state, event, models::PaymentLinkStatus::Cancelled).await;
        }
        "payment_link.expired" => {
            update_payment_link_status(state, event, models::PaymentLinkStatus::Expired).await;
        }

        // --- Refund events ---
        "refund.created" => {
            update_refund_status(state, event, models::RefundStatus::Created).await;
        }
        "refund.processed" => {
            update_refund_status(state, event, models::RefundStatus::Processed).await;
        }
        "refund.failed" => {
            update_refund_status(state, event, models::RefundStatus::Failed).await;
        }

        // --- Unknown events ---
        _ => {
            tracing::debug!(event_type = %event_type, "Unhandled webhook event type");
        }
    }

    Ok(event_type.to_string())
}

async fn update_transaction_by_order_id(
    state: &AppState,
    order_id: &str,
    status: models::TransactionStatus,
) {
    use crate::models::Transaction;

    let filter = mongodb::bson::doc! { "provider_order_id": order_id };
    let update = mongodb::bson::doc! {
        "$set": {
            "status": mongodb::bson::to_bson(&status).unwrap_or_default(),
            "updated_at": mongodb::bson::DateTime::now()
        }
    };

    if let Err(e) = state
        .db
        .collection::<Transaction>("transactions")
        .update_one(filter, update, None)
        .await
    {
        tracing::error!(order_id = %order_id, error = %e, "Failed to update transaction via webhook");
    } else {
        tracing::info!(order_id = %order_id, status = ?status, "Transaction updated via webhook");
    }
}

async fn update_subscription_status(
    state: &AppState,
    event: &WebhookEvent,
    status: models::SubscriptionStatus,
) {
    if let Some(ref sub_entity) = event.payload.subscription {
        let sub = &sub_entity.entity;
        tracing::info!(subscription_id = %sub.id, status = ?status, "Subscription webhook received");

        if let Err(e) = state
            .repository
            .update_subscription_status_by_razorpay_id(&sub.id, status)
            .await
        {
            tracing::error!(error = %e, "Failed to update subscription status");
        }
    }
}

async fn update_account_status(
    state: &AppState,
    event: &WebhookEvent,
    status: models::LinkedAccountStatus,
) {
    if let Some(ref account_entity) = event.payload.account {
        let account = &account_entity.entity;
        tracing::info!(account_id = %account.id, status = ?status, "Account webhook received");

        if let Err(e) = state
            .repository
            .update_linked_account_status_by_razorpay_id(&account.id, status)
            .await
        {
            tracing::error!(error = %e, "Failed to update linked account status");
        }
    }
}

async fn update_payment_link_status(
    state: &AppState,
    event: &WebhookEvent,
    status: models::PaymentLinkStatus,
) {
    if let Some(ref link_entity) = event.payload.payment_link {
        let link = &link_entity.entity;
        tracing::info!(payment_link_id = %link.id, status = ?status, "Payment link webhook received");

        if let Err(e) = state
            .repository
            .update_payment_link_status_by_razorpay_id(&link.id, status)
            .await
        {
            tracing::error!(error = %e, "Failed to update payment link status");
        }
    }
}

async fn update_refund_status(
    state: &AppState,
    event: &WebhookEvent,
    status: models::RefundStatus,
) {
    if let Some(ref refund_entity) = event.payload.refund {
        let refund = &refund_entity.entity;
        tracing::info!(refund_id = %refund.id, status = ?status, "Refund webhook received");

        if let Err(e) = state
            .repository
            .update_refund_status_by_razorpay_id(&refund.id, status)
            .await
        {
            tracing::error!(error = %e, "Failed to update refund status");
        }
    }
}
