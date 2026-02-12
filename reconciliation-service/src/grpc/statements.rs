//! Statement import and management gRPC handlers.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::*;
use crate::services::Database;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn import_statement(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ImportStatementRequest>,
) -> Result<Response<ImportStatementResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_IMPORT)
        .await?;

    let req = request.into_inner();
    tracing::info!(
        bank_account_id = %req.bank_account_id,
        document_id = %req.document_id,
        "Importing statement"
    );

    let statement = db
        .create_statement(&_auth.tenant_id, &req.bank_account_id, &req.document_id)
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to import statement: {}", e))
        })?;

    // TODO: Trigger async GenAI extraction via genai-service

    Ok(Response::new(ImportStatementResponse {
        statement: Some(statement.into()),
    }))
}

pub async fn get_statement(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetStatementRequest>,
) -> Result<Response<GetStatementResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
        .await?;

    let req = request.into_inner();
    let statement = db
        .get_statement(&_auth.tenant_id, &req.statement_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get statement: {}", e)))?
        .ok_or_else(|| Status::not_found("Statement not found"))?;

    Ok(Response::new(GetStatementResponse {
        statement: Some(statement.into()),
    }))
}

pub async fn list_statements(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListStatementsRequest>,
) -> Result<Response<ListStatementsResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
        .await?;

    let req = request.into_inner();
    let (statements, next_token) = db
        .list_statements(
            &_auth.tenant_id,
            &req.bank_account_id,
            req.page_size,
            req.page_token.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to list statements: {}", e)))?;

    Ok(Response::new(ListStatementsResponse {
        statements: statements.into_iter().map(|s| s.into()).collect(),
        next_page_token: next_token,
    }))
}

pub async fn get_staged_transactions(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetStagedTransactionsRequest>,
) -> Result<Response<GetStagedTransactionsResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
        .await?;

    let req = request.into_inner();
    let (transactions, next_token) = db
        .get_staged_transactions(
            &_auth.tenant_id,
            &req.statement_id,
            req.page_size,
            req.page_token.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to get staged transactions: {}", e)))?;

    Ok(Response::new(GetStagedTransactionsResponse {
        transactions: transactions.into_iter().map(|t| t.into()).collect(),
        next_page_token: next_token,
    }))
}

pub async fn update_staged_transaction(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<UpdateStagedTransactionRequest>,
) -> Result<Response<UpdateStagedTransactionResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STAGED_UPDATE)
        .await?;

    let req = request.into_inner();
    let transaction = db
        .update_staged_transaction(
            &_auth.tenant_id,
            &req.transaction_id,
            req.transaction_date.as_deref(),
            req.description.as_deref(),
            req.reference.as_deref(),
            req.amount.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to update staged transaction: {}", e)))?
        .ok_or_else(|| Status::not_found("Transaction not found"))?;

    Ok(Response::new(UpdateStagedTransactionResponse {
        transaction: Some(transaction.into()),
    }))
}

pub async fn commit_statement(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<CommitStatementRequest>,
) -> Result<Response<CommitStatementResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_COMMIT)
        .await?;

    let req = request.into_inner();
    let (statement, count) = db
        .commit_statement(&_auth.tenant_id, &req.statement_id)
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to commit statement: {}", e))
        })?;

    // Apply matching rules to auto-match transactions
    let auto_matched = db
        .apply_matching_rules(&_auth.tenant_id, &req.statement_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to apply matching rules");
            0
        });

    tracing::info!(
        statement_id = %req.statement_id,
        transactions_committed = %count,
        auto_matched = %auto_matched,
        "Statement committed with auto-matching"
    );

    Ok(Response::new(CommitStatementResponse {
        statement: Some(statement.into()),
        transactions_committed: count,
    }))
}

pub async fn abandon_statement(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<AbandonStatementRequest>,
) -> Result<Response<AbandonStatementResponse>, Status> {
    let _auth = capability_checker
        .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_ABANDON)
        .await?;

    let req = request.into_inner();
    db.abandon_statement(&_auth.tenant_id, &req.statement_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to abandon statement: {}", e)))?;

    Ok(Response::new(AbandonStatementResponse { success: true }))
}
