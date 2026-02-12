//! gRPC handlers for balance and statement operations.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::{
    Direction as ProtoDirection, GetBalanceRequest, GetBalanceResponse, GetBalancesRequest,
    GetBalancesResponse, GetStatementRequest, GetStatementResponse,
};
use crate::grpc::service::{format_decimal, parse_tenant_id};
use crate::services::Database;
use chrono::NaiveDate;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{instrument, warn};
use uuid::Uuid;

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "GetBalance")
)]
pub async fn get_balance(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetBalanceRequest>,
) -> Result<Response<GetBalanceResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_BALANCE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
        Status::invalid_argument("Invalid account_id format")
    })?;

    let as_of_date = if req.as_of_date.is_empty() {
        None
    } else {
        Some(
            NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid as_of_date format (expected YYYY-MM-DD)")
            })?,
        )
    };

    let result = db
        .get_balance(tenant_id, account_id, as_of_date)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get balance");
            Status::internal("Failed to get balance")
        })?;

    match result {
        Some((balance, currency)) => {
            Ok(Response::new(GetBalanceResponse {
                account_id: account_id.to_string(),
                balance: format_decimal(&balance),
                currency,
                as_of_date: as_of_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
                    .to_string(),
            }))
        }
        None => {
            Err(Status::not_found("Account not found"))
        }
    }
}

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "GetBalances")
)]
pub async fn get_balances(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetBalancesRequest>,
) -> Result<Response<GetBalancesResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_BALANCE_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let mut account_ids = Vec::with_capacity(req.account_ids.len());
    for id in &req.account_ids {
        let account_id = Uuid::parse_str(id).map_err(|_| {
            Status::invalid_argument("Invalid account_id format")
        })?;
        account_ids.push(account_id);
    }

    let as_of_date = if req.as_of_date.is_empty() {
        None
    } else {
        Some(
            NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid as_of_date format (expected YYYY-MM-DD)")
            })?,
        )
    };

    let results = db
        .get_balances(tenant_id, &account_ids, as_of_date)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get balances");
            Status::internal("Failed to get balances")
        })?;

    let as_of_str = as_of_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive())
        .to_string();

    Ok(Response::new(GetBalancesResponse {
        balances: results
            .iter()
            .map(|(account_id, balance, currency)| GetBalanceResponse {
                account_id: account_id.to_string(),
                balance: format_decimal(balance),
                currency: currency.clone(),
                as_of_date: as_of_str.clone(),
            })
            .collect(),
    }))
}

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "GetStatement")
)]
pub async fn get_statement(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetStatementRequest>,
) -> Result<Response<GetStatementResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_STATEMENT_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
        Status::invalid_argument("Invalid account_id format")
    })?;

    let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
        Status::invalid_argument("Invalid start_date format (expected YYYY-MM-DD)")
    })?;

    let end_date = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
        Status::invalid_argument("Invalid end_date format (expected YYYY-MM-DD)")
    })?;

    if end_date < start_date {
        return Err(Status::invalid_argument("end_date must be >= start_date"));
    }

    let result = db
        .get_statement(tenant_id, account_id, start_date, end_date)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get statement");
            Status::internal("Failed to get statement")
        })?;

    match result {
        Some((currency, opening_balance, closing_balance, entries)) => {
            let mut running = opening_balance;
            let lines: Vec<_> = entries
                .iter()
                .map(|e| {
                    match e.direction.as_str() {
                        "debit" => running += e.amount,
                        "credit" => running -= e.amount,
                        _ => {}
                    }
                    crate::grpc::proto::StatementLine {
                        entry_id: e.entry_id.to_string(),
                        journal_id: e.journal_id.to_string(),
                        effective_date: e.effective_date.to_string(),
                        direction: match e.direction.as_str() {
                            "debit" => ProtoDirection::Debit as i32,
                            "credit" => ProtoDirection::Credit as i32,
                            _ => ProtoDirection::Unspecified as i32,
                        },
                        amount: format_decimal(&e.amount),
                        running_balance: format_decimal(&running),
                        metadata: e
                            .metadata
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_default(),
                    }
                })
                .collect();

            Ok(Response::new(GetStatementResponse {
                account_id: account_id.to_string(),
                currency,
                opening_balance: format_decimal(&opening_balance),
                closing_balance: format_decimal(&closing_balance),
                lines,
            }))
        }
        None => {
            Err(Status::not_found("Account not found"))
        }
    }
}
