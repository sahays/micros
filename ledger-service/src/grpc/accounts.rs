//! gRPC handlers for account operations.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::{
    AccountType as ProtoAccountType, CreateAccountRequest, CreateAccountResponse,
    GetAccountRequest, GetAccountResponse, ListAccountsRequest, ListAccountsResponse,
};
use crate::grpc::service::{account_to_proto, parse_tenant_id};
use crate::models::{AccountType, CreateAccount};
use crate::services::Database;
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "CreateAccount")
)]
pub async fn create_account(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<CreateAccountRequest>,
) -> Result<Response<CreateAccountResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_ACCOUNT_CREATE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_type = AccountType::from_proto(req.account_type).ok_or_else(|| {
        Status::invalid_argument("Invalid account_type")
    })?;

    if req.currency.len() != 3 {
        return Err(Status::invalid_argument(
            "Currency must be a 3-letter ISO 4217 code",
        ));
    }

    if req.account_code.is_empty() || req.account_code.len() > 100 {
        return Err(Status::invalid_argument(
            "account_code must be between 1 and 100 characters",
        ));
    }

    let metadata = if req.metadata.is_empty() {
        None
    } else {
        Some(serde_json::from_str(&req.metadata).map_err(|_| {
            Status::invalid_argument("Invalid metadata JSON")
        })?)
    };

    let input = CreateAccount {
        tenant_id,
        account_type,
        account_code: req.account_code,
        currency: req.currency.to_uppercase(),
        allow_negative: req.allow_negative,
        metadata,
    };

    let account = db.create_account(&input).await.map_err(|e| {
        warn!(error = %e, "Failed to create account");
        match e {
            service_core::error::AppError::Conflict(err) => Status::already_exists(err.to_string()),
            _ => Status::internal("Failed to create account"),
        }
    })?;

    info!(
        account_id = %account.account_id,
        account_code = %account.account_code,
        "Account created successfully"
    );

    Ok(Response::new(CreateAccountResponse {
        account: Some(account_to_proto(&account, Some(Decimal::ZERO))),
    }))
}

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "GetAccount")
)]
pub async fn get_account(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetAccountRequest>,
) -> Result<Response<GetAccountResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_ACCOUNT_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
        Status::invalid_argument("Invalid account_id format")
    })?;

    let result = db
        .get_account_with_balance(tenant_id, account_id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get account");
            Status::internal("Failed to get account")
        })?;

    match result {
        Some((acc, balance)) => {
            Ok(Response::new(GetAccountResponse {
                account: Some(account_to_proto(&acc, Some(balance))),
            }))
        }
        None => {
            Err(Status::not_found("Account not found"))
        }
    }
}

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "ListAccounts")
)]
pub async fn list_accounts(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListAccountsRequest>,
) -> Result<Response<ListAccountsResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_ACCOUNT_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_type = if req.account_type == ProtoAccountType::Unspecified as i32 {
        None
    } else {
        AccountType::from_proto(req.account_type)
    };

    let currency = if req.currency.is_empty() {
        None
    } else {
        Some(req.currency.as_str())
    };

    let page_token = if req.page_token.is_empty() {
        None
    } else {
        Some(Uuid::parse_str(&req.page_token).map_err(|_| {
            Status::invalid_argument("Invalid page_token format")
        })?)
    };

    let page_size = if req.page_size <= 0 {
        20
    } else {
        req.page_size
    };

    let accounts = db
        .list_accounts(tenant_id, account_type, currency, page_size, page_token)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to list accounts");
            Status::internal("Failed to list accounts")
        })?;

    let next_page_token = if accounts.len() == page_size as usize {
        accounts.last().map(|a| a.account_id.to_string())
    } else {
        None
    };

    Ok(Response::new(ListAccountsResponse {
        accounts: accounts.iter().map(|a| account_to_proto(a, None)).collect(),
        next_page_token: next_page_token.unwrap_or_default(),
    }))
}
