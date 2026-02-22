//! LedgerService gRPC implementation -- slim delegation layer.

use crate::grpc::proto::{
    ledger_service_server::LedgerService, Account as ProtoAccount, CreateAccountRequest,
    CreateAccountResponse, Direction as ProtoDirection, GetAccountRequest, GetAccountResponse,
    GetBalanceRequest, GetBalanceResponse, GetBalancesRequest, GetBalancesResponse,
    GetStatementRequest, GetStatementResponse, GetTransactionRequest, GetTransactionResponse,
    LedgerEntry as ProtoLedgerEntry, ListAccountsRequest, ListAccountsResponse,
    ListTransactionsRequest, ListTransactionsResponse, PostTransactionRequest,
    PostTransactionResponse, Transaction as ProtoTransaction,
};
use crate::models::{Account, AccountType, LedgerEntry};
use crate::services::Database;
use prost_types::Timestamp;
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{accounts, reporting, transactions};

// --- Shared helpers (used by handler modules) ---------------------------------

/// Format a Decimal as a normalized string (remove trailing zeros).
pub fn format_decimal(d: &Decimal) -> String {
    let s = d.to_string();
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    } else {
        s
    }
}

/// Convert domain Account to proto Account.
pub fn account_to_proto(account: &Account, balance: Option<Decimal>) -> ProtoAccount {
    ProtoAccount {
        account_id: account.account_id.to_string(),
        tenant_id: account.tenant_id.to_string(),
        account_type: AccountType::from_proto(match account.account_type.as_str() {
            "asset" => 1,
            "liability" => 2,
            "equity" => 3,
            "revenue" => 4,
            "expense" => 5,
            _ => 0,
        })
        .map(|t| t.to_proto())
        .unwrap_or(0),
        account_code: account.account_code.clone(),
        currency: account.currency.clone(),
        allow_negative: account.allow_negative,
        metadata: account
            .metadata
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_default(),
        created_at: Some(Timestamp {
            seconds: account.created_utc.timestamp(),
            nanos: account.created_utc.timestamp_subsec_nanos() as i32,
        }),
        closed_at: account.closed_utc.map(|t| Timestamp {
            seconds: t.timestamp(),
            nanos: t.timestamp_subsec_nanos() as i32,
        }),
        balance: balance.map(|b| format_decimal(&b)).unwrap_or_default(),
    }
}

/// Convert domain LedgerEntry to proto LedgerEntry.
pub fn entry_to_proto(entry: &LedgerEntry) -> ProtoLedgerEntry {
    ProtoLedgerEntry {
        entry_id: entry.entry_id.to_string(),
        journal_id: entry.journal_id.to_string(),
        account_id: entry.account_id.to_string(),
        amount: format_decimal(&entry.amount),
        direction: match entry.direction.as_str() {
            "debit" => ProtoDirection::Debit as i32,
            "credit" => ProtoDirection::Credit as i32,
            _ => ProtoDirection::Unspecified as i32,
        },
        effective_date: entry.effective_date.to_string(),
        posted_at: Some(Timestamp {
            seconds: entry.posted_utc.timestamp(),
            nanos: entry.posted_utc.timestamp_subsec_nanos() as i32,
        }),
        metadata: entry
            .metadata
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_default(),
    }
}

/// Convert entries to a Transaction proto.
pub fn entries_to_transaction(
    tenant_id: Uuid,
    journal_id: Uuid,
    entries: &[LedgerEntry],
) -> ProtoTransaction {
    let effective_date = entries
        .first()
        .map(|e| e.effective_date.to_string())
        .unwrap_or_default();
    let posted_at = entries.first().map(|e| Timestamp {
        seconds: e.posted_utc.timestamp(),
        nanos: e.posted_utc.timestamp_subsec_nanos() as i32,
    });
    let idempotency_key = entries
        .iter()
        .find_map(|e| e.idempotency_key.clone())
        .unwrap_or_default();
    let metadata = entries
        .first()
        .and_then(|e| e.metadata.as_ref())
        .map(|m| m.to_string())
        .unwrap_or_default();

    ProtoTransaction {
        journal_id: journal_id.to_string(),
        tenant_id: tenant_id.to_string(),
        entries: entries.iter().map(entry_to_proto).collect(),
        effective_date,
        posted_at,
        idempotency_key,
        metadata,
    }
}

// --- Service struct -----------------------------------------------------------

/// LedgerService implementation.
pub struct LedgerServiceImpl {
    db: Arc<Database>,
}

impl LedgerServiceImpl {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

// --- Trait delegation ---------------------------------------------------------

#[tonic::async_trait]
impl LedgerService for LedgerServiceImpl {
    async fn create_account(
        &self,
        request: Request<CreateAccountRequest>,
    ) -> Result<Response<CreateAccountResponse>, Status> {
        accounts::create_account(&self.db, request).await
    }

    async fn get_account(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountResponse>, Status> {
        accounts::get_account(&self.db, request).await
    }

    async fn list_accounts(
        &self,
        request: Request<ListAccountsRequest>,
    ) -> Result<Response<ListAccountsResponse>, Status> {
        accounts::list_accounts(&self.db, request).await
    }

    async fn post_transaction(
        &self,
        request: Request<PostTransactionRequest>,
    ) -> Result<Response<PostTransactionResponse>, Status> {
        transactions::post_transaction(&self.db, request).await
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        transactions::get_transaction(&self.db, request).await
    }

    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
        transactions::list_transactions(&self.db, request).await
    }

    async fn get_balance(
        &self,
        request: Request<GetBalanceRequest>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        reporting::get_balance(&self.db, request).await
    }

    async fn get_balances(
        &self,
        request: Request<GetBalancesRequest>,
    ) -> Result<Response<GetBalancesResponse>, Status> {
        reporting::get_balances(&self.db, request).await
    }

    async fn get_statement(
        &self,
        request: Request<GetStatementRequest>,
    ) -> Result<Response<GetStatementResponse>, Status> {
        reporting::get_statement(&self.db, request).await
    }
}
