//! gRPC handlers for transaction operations.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::{
    GetTransactionRequest, GetTransactionResponse, ListTransactionsRequest,
    ListTransactionsResponse, PostTransactionRequest, PostTransactionResponse,
};
use crate::grpc::service::{entries_to_transaction, parse_tenant_id};
use crate::models::{Direction, PostEntry};
use crate::services::Database;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "PostTransaction")
)]
pub async fn post_transaction(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<PostTransactionRequest>,
) -> Result<Response<PostTransactionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_TRANSACTION_CREATE)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    if req.entries.is_empty() {
        return Err(Status::invalid_argument(
            "At least 2 entries required for a transaction",
        ));
    }

    let mut entries = Vec::with_capacity(req.entries.len());
    for proto_entry in &req.entries {
        let account_id = Uuid::parse_str(&proto_entry.account_id).map_err(|_| {
            Status::invalid_argument("Invalid account_id format in entry")
        })?;

        let amount = Decimal::from_str(&proto_entry.amount).map_err(|_| {
            Status::invalid_argument("Invalid amount format in entry")
        })?;

        let direction = Direction::from_proto(proto_entry.direction).ok_or_else(|| {
            Status::invalid_argument("Invalid direction in entry")
        })?;

        entries.push(PostEntry {
            account_id,
            amount,
            direction,
        });
    }

    let effective_date = if req.effective_date.is_empty() {
        chrono::Utc::now().date_naive()
    } else {
        NaiveDate::parse_from_str(&req.effective_date, "%Y-%m-%d").map_err(|_| {
            Status::invalid_argument("Invalid effective_date format (expected YYYY-MM-DD)")
        })?
    };

    let idempotency_key = if req.idempotency_key.is_empty() {
        None
    } else {
        Some(req.idempotency_key.as_str())
    };

    let metadata = if req.metadata.is_empty() {
        None
    } else {
        Some(serde_json::from_str(&req.metadata).map_err(|_| {
            Status::invalid_argument("Invalid metadata JSON")
        })?)
    };

    let (journal_id, inserted_entries, _currency) = db
        .post_transaction(
            tenant_id,
            &entries,
            effective_date,
            idempotency_key,
            metadata,
        )
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to post transaction");
            match e {
                service_core::error::AppError::BadRequest(err) => {
                    Status::invalid_argument(err.to_string())
                }
                _ => Status::internal("Failed to post transaction"),
            }
        })?;

    info!(
        journal_id = %journal_id,
        entry_count = inserted_entries.len(),
        "Transaction posted successfully"
    );

    Ok(Response::new(PostTransactionResponse {
        transaction: Some(entries_to_transaction(
            tenant_id,
            journal_id,
            &inserted_entries,
        )),
    }))
}

#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "GetTransaction")
)]
pub async fn get_transaction(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<GetTransactionRequest>,
) -> Result<Response<GetTransactionResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_TRANSACTION_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let journal_id = Uuid::parse_str(&req.journal_id).map_err(|_| {
        Status::invalid_argument("Invalid journal_id format")
    })?;

    let entries = db
        .get_entries_by_journal(tenant_id, journal_id)
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to get transaction");
            Status::internal("Failed to get transaction")
        })?;

    if entries.is_empty() {
        return Err(Status::not_found("Transaction not found"));
    }

    Ok(Response::new(GetTransactionResponse {
        transaction: Some(entries_to_transaction(tenant_id, journal_id, &entries)),
    }))
}

#[allow(clippy::too_many_lines)]
#[instrument(
    skip(db, capability_checker, request),
    fields(service = "ledger-service", method = "ListTransactions")
)]
pub async fn list_transactions(
    db: &Arc<Database>,
    capability_checker: &Arc<CapabilityChecker>,
    request: Request<ListTransactionsRequest>,
) -> Result<Response<ListTransactionsResponse>, Status> {
    let auth = capability_checker
        .require_capability(&request, capabilities::LEDGER_TRANSACTION_READ)
        .await?;
    let tenant_id = parse_tenant_id(&auth)?;

    let req = request.into_inner();

    let account_id = if req.account_id.is_empty() {
        None
    } else {
        Some(Uuid::parse_str(&req.account_id).map_err(|_| {
            Status::invalid_argument("Invalid account_id format")
        })?)
    };

    let start_date = if req.start_date.is_empty() {
        None
    } else {
        Some(
            NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid start_date format (expected YYYY-MM-DD)")
            })?,
        )
    };

    let end_date = if req.end_date.is_empty() {
        None
    } else {
        Some(
            NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid end_date format (expected YYYY-MM-DD)")
            })?,
        )
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

    let transactions = db
        .list_transactions(
            tenant_id, account_id, start_date, end_date, page_size, page_token,
        )
        .await
        .map_err(|e| {
            warn!(error = %e, "Failed to list transactions");
            Status::internal("Failed to list transactions")
        })?;

    let next_page_token = if transactions.len() == page_size as usize {
        transactions.last().map(|(jid, _)| jid.to_string())
    } else {
        None
    };

    Ok(Response::new(ListTransactionsResponse {
        transactions: transactions
            .iter()
            .map(|(jid, entries)| entries_to_transaction(tenant_id, *jid, entries))
            .collect(),
        next_page_token: next_page_token.unwrap_or_default(),
    }))
}
