//! gRPC handlers for payment link operations.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    check_razorpay_configured, datetime_to_timestamp, extract_tenant_context,
};
use crate::grpc::proto::*;
use crate::models;
use crate::services::metrics::record_payment_link;
use crate::services::razorpay_payment_links;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub async fn create_payment_link(
    state: &AppState,
    request: Request<CreatePaymentLinkRequest>,
) -> Result<Response<CreatePaymentLinkResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_LINK_CREATE)
            .await?;
    }

    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let customer = if req.customer_name.is_some()
        || req.customer_email.is_some()
        || req.customer_phone.is_some()
    {
        Some(razorpay_payment_links::PaymentLinkCustomer {
            name: req.customer_name.clone(),
            email: req.customer_email.clone(),
            contact: req.customer_phone.clone(),
        })
    } else {
        None
    };

    let expire_by = req.expire_by_seconds.map(|secs| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        now.as_secs() + secs as u64
    });

    let rz_request = razorpay_payment_links::CreatePaymentLinkRequest {
        amount: req.amount,
        currency: req.currency.clone(),
        description: req.description.clone(),
        customer,
        expire_by,
    };

    let rz_response = state
        .razorpay
        .create_payment_link(rz_request)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create payment link");
            Status::internal(format!("Failed to create payment link: {}", e))
        })?;

    let now = DateTime::now();
    let link = models::PaymentLink {
        id: Uuid::new_v4().to_string(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        razorpay_payment_link_id: rz_response.id,
        amount: req.amount,
        currency: req.currency,
        description: req.description,
        customer_name: req.customer_name,
        customer_email: req.customer_email,
        customer_phone: req.customer_phone,
        status: models::PaymentLinkStatus::Created,
        short_url: rz_response.short_url,
        amount_paid: 0,
        expire_by: expire_by.map(|ts| DateTime::from_millis(ts as i64 * 1000)),
        created_at: now,
        updated_at: now,
    };

    state
        .repository
        .create_payment_link(link.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to save payment link");
            Status::internal("Failed to save payment link")
        })?;

    record_payment_link(&tenant.app_id, "created");

    Ok(Response::new(CreatePaymentLinkResponse {
        payment_link: Some(payment_link_to_proto(link)),
    }))
}

pub async fn get_payment_link(
    state: &AppState,
    request: Request<GetPaymentLinkRequest>,
) -> Result<Response<GetPaymentLinkResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_LINK_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let link = state
        .repository
        .get_payment_link_in_tenant(&tenant.app_id, &tenant.org_id, &req.payment_link_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch payment link");
            Status::internal("Failed to fetch payment link")
        })?
        .ok_or_else(|| Status::not_found("Payment link not found"))?;

    Ok(Response::new(GetPaymentLinkResponse {
        payment_link: Some(payment_link_to_proto(link)),
    }))
}

pub async fn cancel_payment_link(
    state: &AppState,
    request: Request<CancelPaymentLinkRequest>,
) -> Result<Response<CancelPaymentLinkResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_LINK_CANCEL)
            .await?;
    }

    check_razorpay_configured(&state.razorpay)?;

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let link = state
        .repository
        .get_payment_link_in_tenant(&tenant.app_id, &tenant.org_id, &req.payment_link_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch payment link");
            Status::internal("Failed to fetch payment link")
        })?
        .ok_or_else(|| Status::not_found("Payment link not found"))?;

    // Can only cancel if CREATED or PARTIALLY_PAID
    if link.status != models::PaymentLinkStatus::Created
        && link.status != models::PaymentLinkStatus::PartiallyPaid
    {
        return Err(Status::failed_precondition(
            "Payment link can only be cancelled when in CREATED or PARTIALLY_PAID status",
        ));
    }

    state
        .razorpay
        .cancel_payment_link(&link.razorpay_payment_link_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to cancel payment link");
            Status::internal(format!("Failed to cancel payment link: {}", e))
        })?;

    state
        .repository
        .update_payment_link_status_by_razorpay_id(
            &link.razorpay_payment_link_id,
            models::PaymentLinkStatus::Cancelled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update payment link status");
            Status::internal("Failed to update payment link")
        })?;

    record_payment_link(&tenant.app_id, "cancelled");

    let updated = state
        .repository
        .get_payment_link_in_tenant(&tenant.app_id, &tenant.org_id, &req.payment_link_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch payment link");
            Status::internal("Failed to fetch payment link")
        })?
        .ok_or_else(|| Status::not_found("Payment link not found"))?;

    Ok(Response::new(CancelPaymentLinkResponse {
        payment_link: Some(payment_link_to_proto(updated)),
    }))
}

pub async fn list_payment_links(
    state: &AppState,
    request: Request<ListPaymentLinksRequest>,
) -> Result<Response<ListPaymentLinksResponse>, Status> {
    if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
        state
            .capability_checker
            .require_capability_from_metadata(&metadata, capabilities::PAYMENT_LINK_READ)
            .await?;
    }

    let tenant = extract_tenant_context(&request)?;
    let req = request.into_inner();

    let status_filter = req.status.and_then(proto_to_payment_link_status);
    let limit = req.limit.clamp(1, 100) as i64;
    let offset = req.offset.max(0) as u64;

    let (links, total_count) = state
        .repository
        .list_payment_links_in_tenant(&tenant.app_id, &tenant.org_id, status_filter, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list payment links");
            Status::internal("Failed to list payment links")
        })?;

    Ok(Response::new(ListPaymentLinksResponse {
        payment_links: links.into_iter().map(payment_link_to_proto).collect(),
        total_count,
    }))
}

fn payment_link_to_proto(l: models::PaymentLink) -> PaymentLink {
    PaymentLink {
        id: l.id,
        app_id: l.app_id,
        org_id: l.org_id,
        razorpay_payment_link_id: l.razorpay_payment_link_id,
        amount: l.amount,
        currency: l.currency,
        description: l.description,
        customer_name: l.customer_name,
        customer_email: l.customer_email,
        customer_phone: l.customer_phone,
        status: payment_link_status_to_proto(&l.status).into(),
        short_url: l.short_url,
        amount_paid: l.amount_paid,
        expire_by: l.expire_by.and_then(datetime_to_timestamp),
        created_at: datetime_to_timestamp(l.created_at),
        updated_at: datetime_to_timestamp(l.updated_at),
    }
}

fn payment_link_status_to_proto(s: &models::PaymentLinkStatus) -> PaymentLinkStatus {
    match s {
        models::PaymentLinkStatus::Created => PaymentLinkStatus::Created,
        models::PaymentLinkStatus::PartiallyPaid => PaymentLinkStatus::PartiallyPaid,
        models::PaymentLinkStatus::Paid => PaymentLinkStatus::Paid,
        models::PaymentLinkStatus::Cancelled => PaymentLinkStatus::Cancelled,
        models::PaymentLinkStatus::Expired => PaymentLinkStatus::Expired,
    }
}

fn proto_to_payment_link_status(s: i32) -> Option<models::PaymentLinkStatus> {
    match PaymentLinkStatus::try_from(s) {
        Ok(PaymentLinkStatus::Created) => Some(models::PaymentLinkStatus::Created),
        Ok(PaymentLinkStatus::PartiallyPaid) => Some(models::PaymentLinkStatus::PartiallyPaid),
        Ok(PaymentLinkStatus::Paid) => Some(models::PaymentLinkStatus::Paid),
        Ok(PaymentLinkStatus::Cancelled) => Some(models::PaymentLinkStatus::Cancelled),
        Ok(PaymentLinkStatus::Expired) => Some(models::PaymentLinkStatus::Expired),
        _ => None,
    }
}
