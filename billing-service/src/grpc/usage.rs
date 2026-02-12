//! Usage tracking gRPC handlers.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::ListUsageFilter;
use crate::models::RecordUsage;
use crate::services::Database;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn record_usage(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<RecordUsageRequest>,
) -> Result<Response<RecordUsageResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_USAGE_WRITE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;
    let component_id = parse_uuid(&req.component_id)?;

    tracing::debug!(
        tenant_id = %tenant_id,
        subscription_id = %subscription_id,
        component_id = %component_id,
        quantity = %req.quantity,
        "Recording usage"
    );

    // Validate subscription exists and belongs to tenant
    let subscription = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    subscription.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    let input = RecordUsage {
        subscription_id,
        component_id,
        quantity: parse_decimal(&req.quantity)?,
        timestamp: timestamp_to_datetime(req.timestamp),
        idempotency_key: req.idempotency_key,
        metadata: if req.metadata.is_empty() {
            None
        } else {
            serde_json::from_str(&req.metadata).ok()
        },
    };

    let record = db.record_usage(&input).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to record usage");
        Status::internal(e.to_string())
    })?;

    Ok(Response::new(RecordUsageResponse {
        usage_record: Some(usage_record_to_proto(record)),
    }))
}

pub async fn get_usage(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetUsageRequest>,
) -> Result<Response<GetUsageResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_USAGE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let record_id = parse_uuid(&req.record_id)?;

    tracing::debug!(tenant_id = %tenant_id, record_id = %record_id, "Getting usage");

    let record = db
        .get_usage_record(tenant_id, record_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let record = record.ok_or_else(|| {
        Status::not_found("Usage record not found")
    })?;

    Ok(Response::new(GetUsageResponse {
        usage_record: Some(usage_record_to_proto(record)),
    }))
}

pub async fn list_usage(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListUsageRequest>,
) -> Result<Response<ListUsageResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_USAGE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::debug!(tenant_id = %tenant_id, "Listing usage");

    let filter = ListUsageFilter {
        component_id: if req.component_id.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.component_id)?)
        },
        cycle_id: if req.cycle_id.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.cycle_id)?)
        },
        is_invoiced: if req.is_invoiced { Some(true) } else { None },
        page_size: if req.page_size > 0 { req.page_size } else { 50 },
        page_token: if req.page_token.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.page_token)?)
        },
    };

    let records = db
        .list_usage_records(tenant_id, subscription_id, &filter)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let proto_records: Vec<_> = records.into_iter().map(usage_record_to_proto).collect();
    let next_page_token = proto_records
        .last()
        .map(|r| r.record_id.clone())
        .unwrap_or_default();

    Ok(Response::new(ListUsageResponse {
        usage_records: proto_records,
        next_page_token,
    }))
}

pub async fn get_usage_summary(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetUsageSummaryRequest>,
) -> Result<Response<GetUsageSummaryResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::BILLING_USAGE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;
    let cycle_id = if req.cycle_id.is_empty() {
        None
    } else {
        Some(parse_uuid(&req.cycle_id)?)
    };

    tracing::debug!(
        tenant_id = %tenant_id,
        subscription_id = %subscription_id,
        "Getting usage summary"
    );

    let summaries = db
        .get_usage_summary(tenant_id, subscription_id, cycle_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let proto_summaries: Vec<_> = summaries
        .into_iter()
        .map(|s| UsageComponentSummary {
            component_id: s.component_id.to_string(),
            name: s.name,
            total_quantity: s.total_quantity.to_string(),
            included_units: s.included_units,
            billable_units: s.billable_units.to_string(),
            amount: s.amount.to_string(),
        })
        .collect();

    Ok(Response::new(GetUsageSummaryResponse {
        component_summaries: proto_summaries,
    }))
}
