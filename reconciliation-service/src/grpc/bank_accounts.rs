//! Bank account management gRPC handlers.

use crate::grpc::proto::*;
use crate::services::Database;
use service_core::grpc::proto::ledger::AccountType as LedgerAccountType;
use service_core::grpc::LedgerClient;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn register_bank_account(
    db: &Arc<Database>,
    ledger_client: &Option<Arc<LedgerClient>>,
    request: Request<RegisterBankAccountRequest>,
) -> Result<Response<RegisterBankAccountResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    tracing::info!(
        bank_name = %req.bank_name,
        ledger_account_id = %req.ledger_account_id,
        "Registering bank account"
    );

    // Check for duplicate ledger_account_id
    let existing = db
        .get_bank_account_by_ledger_id(&app_id, &req.ledger_account_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to check existing account: {}", e)))?;

    if existing.is_some() {
        return Err(Status::already_exists(
            "A bank account with this ledger_account_id already exists",
        ));
    }

    // Validate ledger account exists and is asset type
    if let Some(ref ledger_client) = ledger_client {
        let ledger_response = ledger_client
            .get_account(&app_id, &req.ledger_account_id)
            .await
            .map_err(|e| {
                if e.code() == tonic::Code::NotFound {
                    Status::not_found("Ledger account not found")
                } else {
                    Status::internal(format!("Failed to validate ledger account: {}", e))
                }
            })?;

        let ledger_account = ledger_response
            .account
            .ok_or_else(|| Status::not_found("Ledger account not found"))?;

        // Validate account type is asset (cash/bank accounts are assets)
        if ledger_account.account_type != LedgerAccountType::Asset as i32 {
            return Err(Status::invalid_argument(
                "Ledger account must be an asset type (cash/bank account)",
            ));
        }

        // Validate currency matches
        if ledger_account.currency != req.currency {
            return Err(Status::invalid_argument(format!(
                "Currency mismatch: bank account currency '{}' does not match ledger account currency '{}'",
                req.currency, ledger_account.currency
            )));
        }
    }

    let bank_account = db
        .create_bank_account(
            &app_id,
            &req.ledger_account_id,
            &req.bank_name,
            &req.account_number_masked,
            &req.currency,
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to create bank account: {}", e)))?;

    Ok(Response::new(RegisterBankAccountResponse {
        bank_account: Some(bank_account.into()),
    }))
}

pub async fn get_bank_account(
    db: &Arc<Database>,
    request: Request<GetBankAccountRequest>,
) -> Result<Response<GetBankAccountResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let bank_account = db
        .get_bank_account(&app_id, &req.bank_account_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get bank account: {}", e)))?
        .ok_or_else(|| Status::not_found("Bank account not found"))?;

    Ok(Response::new(GetBankAccountResponse {
        bank_account: Some(bank_account.into()),
    }))
}

pub async fn list_bank_accounts(
    db: &Arc<Database>,
    request: Request<ListBankAccountsRequest>,
) -> Result<Response<ListBankAccountsResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let (accounts, next_token) = db
        .list_bank_accounts(&app_id, req.page_size, req.page_token.as_deref())
        .await
        .map_err(|e| Status::internal(format!("Failed to list bank accounts: {}", e)))?;

    Ok(Response::new(ListBankAccountsResponse {
        bank_accounts: accounts.into_iter().map(|a| a.into()).collect(),
        next_page_token: next_token,
    }))
}

pub async fn update_bank_account(
    db: &Arc<Database>,
    request: Request<UpdateBankAccountRequest>,
) -> Result<Response<UpdateBankAccountResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let bank_account = db
        .update_bank_account(
            &app_id,
            &req.bank_account_id,
            req.bank_name.as_deref(),
            req.account_number_masked.as_deref(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to update bank account: {}", e)))?
        .ok_or_else(|| Status::not_found("Bank account not found"))?;

    Ok(Response::new(UpdateBankAccountResponse {
        bank_account: Some(bank_account.into()),
    }))
}
