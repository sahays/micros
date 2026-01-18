//! Transaction handlers with multi-tenant support.
//!
//! All operations are scoped to the tenant (app_id, org_id) from the request context.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use mongodb::bson::DateTime;
use service_core::error::AppError;
use uuid::Uuid;

use crate::{
    dtos::{CreateTransactionRequest, TransactionResponse, UpdateTransactionStatusRequest},
    middleware::TenantContext,
    models::{Transaction, TransactionStatus},
    AppState,
};

/// Create a new transaction within the tenant's scope.
pub async fn create_transaction(
    State(state): State<AppState>,
    tenant: TenantContext,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), AppError> {
    let now = DateTime::now();
    let transaction = Transaction {
        id: Uuid::new_v4(),
        app_id: tenant.app_id.clone(),
        org_id: tenant.org_id.clone(),
        user_id: tenant.user_id.clone(),
        amount: payload.amount,
        currency: payload.currency,
        status: TransactionStatus::Created,
        provider_order_id: None,
        created_at: now,
        updated_at: now,
    };

    tracing::info!(
        transaction_id = %transaction.id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        amount = payload.amount,
        "Creating transaction"
    );

    state
        .repository
        .create_transaction(transaction.clone())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(TransactionResponse::from(transaction)),
    ))
}

/// Get a transaction by ID within the tenant's scope.
pub async fn get_transaction(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(transaction_id): Path<Uuid>,
) -> Result<Json<TransactionResponse>, AppError> {
    tracing::info!(
        transaction_id = %transaction_id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        "Fetching transaction"
    );

    let transaction = state
        .repository
        .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, transaction_id)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Transaction not found")))?;

    Ok(Json(TransactionResponse::from(transaction)))
}

/// Update transaction status within the tenant's scope.
pub async fn update_transaction_status(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(transaction_id): Path<Uuid>,
    Json(payload): Json<UpdateTransactionStatusRequest>,
) -> Result<StatusCode, AppError> {
    tracing::info!(
        transaction_id = %transaction_id,
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        new_status = ?payload.status,
        "Updating transaction status"
    );

    // Verify transaction exists within tenant scope
    let _transaction = state
        .repository
        .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, transaction_id)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Transaction not found")))?;

    state
        .repository
        .update_transaction_status_in_tenant(
            &tenant.app_id,
            &tenant.org_id,
            transaction_id,
            payload.status,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
