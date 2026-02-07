//! Shared gRPC helper functions.

use crate::middleware::TenantContext;
use crate::models::{Transaction, TransactionStatus};
use mongodb::bson::DateTime;
use prost_types::Timestamp;
use tonic::{Request, Status};

use super::proto::{Transaction as ProtoTransaction, TransactionStatus as ProtoTransactionStatus};

/// Extract tenant context from gRPC metadata.
#[allow(clippy::result_large_err)]
pub fn extract_tenant_context(
    request: &Request<impl std::any::Any>,
) -> Result<TenantContext, Status> {
    let metadata = request.metadata();

    let app_id = metadata
        .get("x-app-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;

    let org_id = metadata
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| Status::unauthenticated("Missing x-org-id header"))?;

    let user_id = metadata
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    Ok(TenantContext::new(app_id, org_id, user_id))
}

/// Convert MongoDB DateTime to protobuf Timestamp.
pub fn datetime_to_timestamp(dt: DateTime) -> Option<Timestamp> {
    let millis = dt.timestamp_millis();
    Some(Timestamp {
        seconds: millis / 1000,
        nanos: ((millis % 1000) * 1_000_000) as i32,
    })
}

/// Convert model Transaction to proto Transaction.
pub fn transaction_to_proto(t: Transaction) -> ProtoTransaction {
    ProtoTransaction {
        id: t.id.to_string(),
        app_id: t.app_id,
        org_id: t.org_id,
        user_id: t.user_id,
        amount_paise: t.amount_paise,
        currency: t.currency,
        status: status_to_proto(t.status).into(),
        provider_order_id: t.provider_order_id,
        linked_account_id: t.linked_account_id,
        subscription_id: t.subscription_id,
        payment_link_id: t.payment_link_id,
        created_at: datetime_to_timestamp(t.created_at),
        updated_at: datetime_to_timestamp(t.updated_at),
    }
}

/// Convert model TransactionStatus to proto TransactionStatus.
pub fn status_to_proto(status: TransactionStatus) -> ProtoTransactionStatus {
    match status {
        TransactionStatus::Created => ProtoTransactionStatus::Created,
        TransactionStatus::Pending => ProtoTransactionStatus::Pending,
        TransactionStatus::Completed => ProtoTransactionStatus::Completed,
        TransactionStatus::Failed => ProtoTransactionStatus::Failed,
        TransactionStatus::Refunded => ProtoTransactionStatus::Refunded,
        TransactionStatus::PartiallyRefunded => ProtoTransactionStatus::PartiallyRefunded,
    }
}

/// Convert proto TransactionStatus to model TransactionStatus.
pub fn proto_to_status(status: i32) -> Option<TransactionStatus> {
    match ProtoTransactionStatus::try_from(status) {
        Ok(ProtoTransactionStatus::Created) => Some(TransactionStatus::Created),
        Ok(ProtoTransactionStatus::Pending) => Some(TransactionStatus::Pending),
        Ok(ProtoTransactionStatus::Completed) => Some(TransactionStatus::Completed),
        Ok(ProtoTransactionStatus::Failed) => Some(TransactionStatus::Failed),
        Ok(ProtoTransactionStatus::Refunded) => Some(TransactionStatus::Refunded),
        Ok(ProtoTransactionStatus::PartiallyRefunded) => Some(TransactionStatus::PartiallyRefunded),
        _ => None,
    }
}

/// Check that a feature flag is enabled.
#[allow(clippy::result_large_err)]
pub fn check_feature_flag(enabled: bool, name: &str) -> Result<(), Status> {
    if !enabled {
        return Err(Status::failed_precondition(format!(
            "{} is not enabled",
            name
        )));
    }
    Ok(())
}

/// Check that Razorpay client is configured.
#[allow(clippy::result_large_err)]
pub fn check_razorpay_configured(razorpay: &crate::services::RazorpayClient) -> Result<(), Status> {
    if !razorpay.is_configured() {
        return Err(Status::failed_precondition(
            "Razorpay is not configured for this environment",
        ));
    }
    Ok(())
}
