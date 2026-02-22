//! Billing run gRPC handlers.

use crate::grpc::helpers::*;
use crate::grpc::proto::*;
use crate::models::{
    BillingCycleStatus, BillingRunStatus, BillingRunType, ChargeType, CreateCharge,
    ListBillingRunsFilter, SubscriptionStatus,
};
use crate::services::Database;
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn run_billing(
    db: &Arc<Database>,
    request: Request<RunBillingRequest>,
) -> Result<Response<RunBillingResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    let run_type = BillingRunType::from_proto(req.run_type);

    tracing::info!(tenant_id = %tenant_id, run_type = ?run_type, "Starting billing run");

    // Create billing run record
    let billing_run = db
        .create_billing_run(tenant_id, run_type)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    // Find subscriptions due for billing
    let subscriptions = db
        .find_subscriptions_due_for_billing(tenant_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let mut processed = 0;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut results = Vec::new();

    for subscription in subscriptions {
        processed += 1;

        // Get current billing cycle
        let cycle = match db
            .get_current_billing_cycle(subscription.subscription_id)
            .await
        {
            Ok(Some(c)) => c,
            Ok(None) => {
                failed += 1;
                let result = db
                    .create_billing_run_result(
                        billing_run.run_id,
                        subscription.subscription_id,
                        "failed",
                        None,
                        Some("No pending billing cycle".to_string()),
                    )
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                results.push(billing_run_result_to_proto(result));
                continue;
            }
            Err(e) => {
                failed += 1;
                let result = db
                    .create_billing_run_result(
                        billing_run.run_id,
                        subscription.subscription_id,
                        "failed",
                        None,
                        Some(e.to_string()),
                    )
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                results.push(billing_run_result_to_proto(result));
                continue;
            }
        };

        // Get plan for recurring charge
        let plan = match db.get_plan(tenant_id, subscription.plan_id).await {
            Ok(Some(p)) => p,
            Ok(None) | Err(_) => {
                failed += 1;
                let result = db
                    .create_billing_run_result(
                        billing_run.run_id,
                        subscription.subscription_id,
                        "failed",
                        None,
                        Some("Plan not found".to_string()),
                    )
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                results.push(billing_run_result_to_proto(result));
                continue;
            }
        };

        // Create recurring charge
        let recurring_input = CreateCharge {
            cycle_id: cycle.cycle_id,
            charge_type: ChargeType::Recurring,
            description: format!("Monthly subscription - {}", plan.name),
            quantity: Decimal::ONE,
            unit_price: plan.base_price,
            amount: plan.base_price,
            is_prorated: false,
            proration_factor: None,
            component_id: None,
            metadata: None,
        };

        if let Err(e) = db.create_charge(&recurring_input).await {
            tracing::error!(error = %e, "Failed to create recurring charge");
        }

        // Create usage charges
        let usage_summaries = db
            .get_usage_summary(
                tenant_id,
                subscription.subscription_id,
                Some(cycle.cycle_id),
            )
            .await
            .unwrap_or_default();

        for summary in usage_summaries {
            if summary.billable_units > Decimal::ZERO {
                let usage_input = CreateCharge {
                    cycle_id: cycle.cycle_id,
                    charge_type: ChargeType::Usage,
                    description: format!(
                        "{} - {} billable units",
                        summary.name, summary.billable_units
                    ),
                    quantity: summary.billable_units,
                    unit_price: summary.amount / summary.billable_units,
                    amount: summary.amount,
                    is_prorated: false,
                    proration_factor: None,
                    component_id: Some(summary.component_id),
                    metadata: None,
                };

                if let Err(e) = db.create_charge(&usage_input).await {
                    tracing::error!(error = %e, "Failed to create usage charge");
                }
            }
        }

        // Mark cycle as invoiced (invoice creation would be via invoicing-service)
        if let Err(e) = db
            .update_billing_cycle_status(cycle.cycle_id, BillingCycleStatus::Invoiced, None)
            .await
        {
            tracing::error!(error = %e, "Failed to update cycle status");
        }

        // Mark usage as invoiced
        if let Err(e) = db.mark_usage_invoiced(cycle.cycle_id).await {
            tracing::error!(error = %e, "Failed to mark usage as invoiced");
        }

        succeeded += 1;
        let result = db
            .create_billing_run_result(
                billing_run.run_id,
                subscription.subscription_id,
                "success",
                None, // Invoice ID would come from invoicing-service
                None,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        results.push(billing_run_result_to_proto(result));
    }

    // Update billing run with final status
    let status = if failed == 0 {
        BillingRunStatus::Completed
    } else if succeeded == 0 {
        BillingRunStatus::Failed
    } else {
        BillingRunStatus::Completed
    };

    let billing_run = db
        .update_billing_run(
            billing_run.run_id,
            status,
            processed,
            succeeded,
            failed,
            None,
        )
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let billing_run = billing_run.ok_or_else(|| {
        Status::internal("Failed to update billing run")
    })?;

    Ok(Response::new(RunBillingResponse {
        billing_run: Some(billing_run_to_proto(billing_run, results)),
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn run_billing_for_subscription(
    db: &Arc<Database>,
    request: Request<RunBillingForSubscriptionRequest>,
) -> Result<Response<RunBillingForSubscriptionResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    let subscription_id = parse_uuid(&req.subscription_id)?;

    tracing::info!(
        tenant_id = %tenant_id,
        subscription_id = %subscription_id,
        "Running billing for subscription"
    );

    // Create billing run
    let billing_run = db
        .create_billing_run(tenant_id, BillingRunType::Single)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    // Get subscription
    let subscription = db
        .get_subscription(tenant_id, subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let subscription = subscription.ok_or_else(|| {
        Status::not_found("Subscription not found")
    })?;

    if subscription.status != SubscriptionStatus::Active.as_str() {
        return Err(Status::failed_precondition("Subscription must be active"));
    }

    // Get current billing cycle
    let cycle = db
        .get_current_billing_cycle(subscription_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let cycle = cycle.ok_or_else(|| {
        Status::failed_precondition("No pending billing cycle")
    })?;

    // Get plan
    let plan = db
        .get_plan(tenant_id, subscription.plan_id)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let plan = plan.ok_or_else(|| {
        Status::internal("Plan not found")
    })?;

    // Create recurring charge
    let recurring_input = CreateCharge {
        cycle_id: cycle.cycle_id,
        charge_type: ChargeType::Recurring,
        description: format!("Monthly subscription - {}", plan.name),
        quantity: Decimal::ONE,
        unit_price: plan.base_price,
        amount: plan.base_price,
        is_prorated: false,
        proration_factor: None,
        component_id: None,
        metadata: None,
    };

    db.create_charge(&recurring_input).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    // Create usage charges
    let usage_summaries = db
        .get_usage_summary(tenant_id, subscription_id, Some(cycle.cycle_id))
        .await
        .unwrap_or_default();

    for summary in usage_summaries {
        if summary.billable_units > Decimal::ZERO {
            let usage_input = CreateCharge {
                cycle_id: cycle.cycle_id,
                charge_type: ChargeType::Usage,
                description: format!(
                    "{} - {} billable units",
                    summary.name, summary.billable_units
                ),
                quantity: summary.billable_units,
                unit_price: summary.amount / summary.billable_units,
                amount: summary.amount,
                is_prorated: false,
                proration_factor: None,
                component_id: Some(summary.component_id),
                metadata: None,
            };

            db.create_charge(&usage_input).await.map_err(|e| {
                Status::internal(e.to_string())
            })?;
        }
    }

    // Update cycle status
    db.update_billing_cycle_status(cycle.cycle_id, BillingCycleStatus::Invoiced, None)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    // Mark usage as invoiced
    db.mark_usage_invoiced(cycle.cycle_id).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    // Create result
    let result = db
        .create_billing_run_result(billing_run.run_id, subscription_id, "success", None, None)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    // Update billing run
    db.update_billing_run(
        billing_run.run_id,
        BillingRunStatus::Completed,
        1,
        1,
        0,
        None,
    )
    .await
    .map_err(|e| {
        Status::internal(e.to_string())
    })?;

    Ok(Response::new(RunBillingForSubscriptionResponse {
        result: Some(billing_run_result_to_proto(result)),
    }))
}

pub async fn get_billing_run(
    db: &Arc<Database>,
    request: Request<GetBillingRunRequest>,
) -> Result<Response<GetBillingRunResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    let run_id = parse_uuid(&req.run_id)?;

    tracing::debug!(tenant_id = %tenant_id, run_id = %run_id, "Getting billing run");

    let billing_run = db.get_billing_run(tenant_id, run_id).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    let billing_run = billing_run.ok_or_else(|| {
        Status::not_found("Billing run not found")
    })?;

    let results = db.get_billing_run_results(run_id).await.map_err(|e| {
        Status::internal(e.to_string())
    })?;

    Ok(Response::new(GetBillingRunResponse {
        billing_run: Some(billing_run_to_proto(
            billing_run,
            results
                .into_iter()
                .map(billing_run_result_to_proto)
                .collect(),
        )),
    }))
}

pub async fn list_billing_runs(
    db: &Arc<Database>,
    request: Request<ListBillingRunsRequest>,
) -> Result<Response<ListBillingRunsResponse>, Status> {
    let tenant_id = service_core::grpc::extract_app_id_uuid(&request)?;

    let req = request.into_inner();
    tracing::debug!(tenant_id = %tenant_id, "Listing billing runs");

    let filter = ListBillingRunsFilter {
        status: if req.status == 0 {
            None
        } else {
            Some(BillingRunStatus::from_proto(req.status))
        },
        run_type: if req.run_type == 0 {
            None
        } else {
            Some(BillingRunType::from_proto(req.run_type))
        },
        page_size: if req.page_size > 0 { req.page_size } else { 50 },
        page_token: if req.page_token.is_empty() {
            None
        } else {
            Some(parse_uuid(&req.page_token)?)
        },
    };

    let runs = db
        .list_billing_runs(tenant_id, &filter)
        .await
        .map_err(|e| {
            Status::internal(e.to_string())
        })?;

    let proto_runs: Vec<_> = runs
        .into_iter()
        .map(|r| billing_run_to_proto(r, vec![]))
        .collect();
    let next_page_token = proto_runs
        .last()
        .map(|r| r.run_id.clone())
        .unwrap_or_default();

    Ok(Response::new(ListBillingRunsResponse {
        billing_runs: proto_runs,
        next_page_token,
    }))
}
