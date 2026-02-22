//! Reconciliation process and adjustment gRPC handlers.

use crate::grpc::proto::*;
use crate::services::Database;
use service_core::grpc::LedgerClient;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn start_reconciliation(
    db: &Arc<Database>,
    ledger_client: &Option<Arc<LedgerClient>>,
    request: Request<StartReconciliationRequest>,
) -> Result<Response<StartReconciliationResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();

    // Validate bank account exists and belongs to tenant
    let bank_account = db
        .get_bank_account(&app_id, &req.bank_account_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get bank account: {}", e)))?
        .ok_or_else(|| Status::not_found("Bank account not found"))?;

    // Get expected balance from ledger if client is available
    let expected_balance = if let Some(ref ledger_client) = ledger_client {
        match ledger_client
            .get_balance(
                &app_id,
                &bank_account.ledger_account_id.to_string(),
                Some(&req.period_end),
            )
            .await
        {
            Ok(response) => {
                tracing::info!(
                    balance = %response.balance,
                    as_of = %response.as_of_date,
                    "Retrieved expected balance from ledger"
                );
                Some(response.balance)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get balance from ledger, proceeding without");
                None
            }
        }
    } else {
        None
    };

    tracing::info!(
        bank_account_id = %req.bank_account_id,
        period_start = %req.period_start,
        period_end = %req.period_end,
        expected_balance = ?expected_balance,
        "Starting reconciliation"
    );

    let reconciliation = db
        .start_reconciliation(
            &app_id,
            &req.bank_account_id,
            &req.period_start,
            &req.period_end,
            expected_balance.as_deref(),
        )
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to start reconciliation: {}", e))
        })?;

    Ok(Response::new(StartReconciliationResponse {
        reconciliation: Some(reconciliation.into()),
    }))
}

pub async fn get_reconciliation(
    db: &Arc<Database>,
    request: Request<GetReconciliationRequest>,
) -> Result<Response<GetReconciliationResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let reconciliation = db
        .get_reconciliation(&app_id, &req.reconciliation_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get reconciliation: {}", e)))?
        .ok_or_else(|| Status::not_found("Reconciliation not found"))?;

    Ok(Response::new(GetReconciliationResponse {
        reconciliation: Some(reconciliation.into()),
    }))
}

pub async fn list_reconciliations(
    db: &Arc<Database>,
    request: Request<ListReconciliationsRequest>,
) -> Result<Response<ListReconciliationsResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let (reconciliations, next_token) = db
        .list_reconciliations(
            &app_id,
            &req.bank_account_id,
            req.page_size,
            req.page_token.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to list reconciliations: {}", e)))?;

    Ok(Response::new(ListReconciliationsResponse {
        reconciliations: reconciliations.into_iter().map(|r| r.into()).collect(),
        next_page_token: next_token,
    }))
}

pub async fn complete_reconciliation(
    db: &Arc<Database>,
    request: Request<CompleteReconciliationRequest>,
) -> Result<Response<CompleteReconciliationResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let reconciliation = db
        .complete_reconciliation(&app_id, &req.reconciliation_id)
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to complete reconciliation: {}", e))
        })?;

    Ok(Response::new(CompleteReconciliationResponse {
        reconciliation: Some(reconciliation.into()),
    }))
}

pub async fn abandon_reconciliation(
    db: &Arc<Database>,
    request: Request<AbandonReconciliationRequest>,
) -> Result<Response<AbandonReconciliationResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    db.abandon_reconciliation(&app_id, &req.reconciliation_id)
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to abandon reconciliation: {}", e))
        })?;

    Ok(Response::new(AbandonReconciliationResponse {
        success: true,
    }))
}

pub async fn create_adjustment(
    db: &Arc<Database>,
    request: Request<CreateAdjustmentRequest>,
) -> Result<Response<CreateAdjustmentResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();

    // Validate adjustment type
    let adjustment_type = AdjustmentType::try_from(req.adjustment_type)
        .map_err(|_| Status::invalid_argument("Invalid adjustment_type"))?;

    // Validate description is not empty
    if req.description.trim().is_empty() {
        return Err(Status::invalid_argument("Description cannot be empty"));
    }

    // Validate reconciliation exists and is in progress
    let reconciliation = db
        .get_reconciliation(&app_id, &req.reconciliation_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get reconciliation: {}", e)))?
        .ok_or_else(|| Status::not_found("Reconciliation not found"))?;

    if reconciliation.status != "in_progress" {
        return Err(Status::failed_precondition(format!(
            "Cannot add adjustment to reconciliation with status '{}'",
            reconciliation.status
        )));
    }

    tracing::info!(
        reconciliation_id = %req.reconciliation_id,
        adjustment_type = ?adjustment_type,
        amount = %req.amount,
        "Creating adjustment"
    );

    let adjustment = db
        .create_adjustment(
            &app_id,
            &req.reconciliation_id,
            adjustment_type,
            &req.description,
            &req.amount,
        )
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to create adjustment: {}", e))
        })?;

    Ok(Response::new(CreateAdjustmentResponse {
        adjustment: Some(adjustment.into()),
    }))
}

pub async fn list_adjustments(
    db: &Arc<Database>,
    request: Request<ListAdjustmentsRequest>,
) -> Result<Response<ListAdjustmentsResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let (adjustments, next_token) = db
        .list_adjustments(
            &app_id,
            &req.reconciliation_id,
            req.page_size,
            req.page_token.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to list adjustments: {}", e)))?;

    Ok(Response::new(ListAdjustmentsResponse {
        adjustments: adjustments.into_iter().map(|a| a.into()).collect(),
        next_page_token: next_token,
    }))
}
