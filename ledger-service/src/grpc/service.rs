//! LedgerService gRPC implementation.

use crate::grpc::proto::{
    ledger_service_server::LedgerService, Account as ProtoAccount, AccountType as ProtoAccountType,
    CreateAccountRequest, CreateAccountResponse, Direction as ProtoDirection, GetAccountRequest,
    GetAccountResponse, GetBalanceRequest, GetBalanceResponse, GetBalancesRequest,
    GetBalancesResponse, GetStatementRequest, GetStatementResponse, GetTransactionRequest,
    GetTransactionResponse, LedgerEntry as ProtoLedgerEntry, ListAccountsRequest,
    ListAccountsResponse, ListTransactionsRequest, ListTransactionsResponse,
    PostTransactionRequest, PostTransactionResponse, Transaction as ProtoTransaction,
};
use crate::models::{Account, AccountType, CreateAccount, Direction, LedgerEntry, PostEntry};
use crate::services::metrics::{
    ACCOUNTS_CREATED, AMOUNT_TOTAL, ENTRIES_TOTAL, GRPC_REQUESTS_TOTAL, GRPC_REQUEST_DURATION,
    TRANSACTIONS_TOTAL,
};
use crate::services::Database;
use chrono::NaiveDate;
use prost_types::Timestamp;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// LedgerService implementation.
pub struct LedgerServiceImpl {
    db: Arc<Database>,
}

impl LedgerServiceImpl {
    /// Create a new LedgerService instance.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Convert domain Account to proto Account.
    /// P1: Now accepts optional balance to include in response.
    fn account_to_proto(account: &Account, balance: Option<Decimal>) -> ProtoAccount {
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
            balance: balance.map(|b| b.to_string()).unwrap_or_default(),
        }
    }

    /// Convert domain LedgerEntry to proto LedgerEntry.
    fn entry_to_proto(entry: &LedgerEntry) -> ProtoLedgerEntry {
        ProtoLedgerEntry {
            entry_id: entry.entry_id.to_string(),
            journal_id: entry.journal_id.to_string(),
            account_id: entry.account_id.to_string(),
            amount: entry.amount.to_string(),
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
    fn entries_to_transaction(
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
            entries: entries.iter().map(Self::entry_to_proto).collect(),
            effective_date,
            posted_at,
            idempotency_key,
            metadata,
        }
    }
}

#[tonic::async_trait]
impl LedgerService for LedgerServiceImpl {
    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "CreateAccount")
    )]
    async fn create_account(
        &self,
        request: Request<CreateAccountRequest>,
    ) -> Result<Response<CreateAccountResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["CreateAccount"])
            .start_timer();

        let req = request.into_inner();

        // Parse tenant_id
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateAccount", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        // Parse account type
        let account_type = AccountType::from_proto(req.account_type).ok_or_else(|| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateAccount", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid account_type")
        })?;

        // Validate currency (ISO 4217)
        if req.currency.len() != 3 {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateAccount", "invalid_argument"])
                .inc();
            return Err(Status::invalid_argument(
                "Currency must be a 3-letter ISO 4217 code",
            ));
        }

        // Validate account code
        if req.account_code.is_empty() || req.account_code.len() > 100 {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateAccount", "invalid_argument"])
                .inc();
            return Err(Status::invalid_argument(
                "account_code must be between 1 and 100 characters",
            ));
        }

        // Parse metadata if provided
        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["CreateAccount", "invalid_argument"])
                    .inc();
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

        let account = self.db.create_account(&input).await.map_err(|e| {
            warn!(error = %e, "Failed to create account");
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateAccount", "error"])
                .inc();
            match e {
                service_core::error::AppError::Conflict(err) => {
                    Status::already_exists(err.to_string())
                }
                _ => Status::internal("Failed to create account"),
            }
        })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["CreateAccount", "ok"])
            .inc();
        ACCOUNTS_CREATED
            .with_label_values(&[account.account_type.as_str()])
            .inc();

        timer.observe_duration();

        info!(
            account_id = %account.account_id,
            account_code = %account.account_code,
            "Account created successfully"
        );

        Ok(Response::new(CreateAccountResponse {
            account: Some(Self::account_to_proto(&account, Some(Decimal::ZERO))),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "GetAccount")
    )]
    async fn get_account(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetAccount"])
            .start_timer();

        let req = request.into_inner();

        // Parse IDs
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetAccount", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetAccount", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid account_id format")
        })?;

        // P1: Use get_account_with_balance to return balance
        let result = self
            .db
            .get_account_with_balance(tenant_id, account_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get account");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetAccount", "error"])
                    .inc();
                Status::internal("Failed to get account")
            })?;

        timer.observe_duration();

        match result {
            Some((acc, balance)) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetAccount", "ok"])
                    .inc();
                Ok(Response::new(GetAccountResponse {
                    account: Some(Self::account_to_proto(&acc, Some(balance))),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetAccount", "not_found"])
                    .inc();
                Err(Status::not_found("Account not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "ListAccounts")
    )]
    async fn list_accounts(
        &self,
        request: Request<ListAccountsRequest>,
    ) -> Result<Response<ListAccountsResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["ListAccounts"])
            .start_timer();

        let req = request.into_inner();

        // Parse tenant_id
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["ListAccounts", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        // Parse optional account_type filter
        let account_type = if req.account_type == ProtoAccountType::Unspecified as i32 {
            None
        } else {
            AccountType::from_proto(req.account_type)
        };

        // Parse optional currency filter
        let currency = if req.currency.is_empty() {
            None
        } else {
            Some(req.currency.as_str())
        };

        // Parse page token
        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListAccounts", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid page_token format")
            })?)
        };

        let page_size = if req.page_size <= 0 {
            20
        } else {
            req.page_size
        };

        let accounts = self
            .db
            .list_accounts(tenant_id, account_type, currency, page_size, page_token)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to list accounts");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListAccounts", "error"])
                    .inc();
                Status::internal("Failed to list accounts")
            })?;

        timer.observe_duration();

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["ListAccounts", "ok"])
            .inc();

        // Generate next page token if we got a full page
        let next_page_token = if accounts.len() == page_size as usize {
            accounts.last().map(|a| a.account_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListAccountsResponse {
            // Note: Balance is None for list queries - use GetBalances for balance info
            accounts: accounts
                .iter()
                .map(|a| Self::account_to_proto(a, None))
                .collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "PostTransaction")
    )]
    async fn post_transaction(
        &self,
        request: Request<PostTransactionRequest>,
    ) -> Result<Response<PostTransactionResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["PostTransaction"])
            .start_timer();

        let req = request.into_inner();

        // Parse tenant_id
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["PostTransaction", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        // Validate entries exist
        if req.entries.is_empty() {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["PostTransaction", "invalid_argument"])
                .inc();
            return Err(Status::invalid_argument(
                "At least 2 entries required for a transaction",
            ));
        }

        // Parse entries
        let mut entries = Vec::with_capacity(req.entries.len());
        for proto_entry in &req.entries {
            let account_id = Uuid::parse_str(&proto_entry.account_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid account_id format in entry")
            })?;

            let amount = Decimal::from_str(&proto_entry.amount).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid amount format in entry")
            })?;

            let direction = Direction::from_proto(proto_entry.direction).ok_or_else(|| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid direction in entry")
            })?;

            entries.push(PostEntry {
                account_id,
                amount,
                direction,
            });
        }

        // Parse effective date (defaults to today)
        let effective_date = if req.effective_date.is_empty() {
            chrono::Utc::now().date_naive()
        } else {
            NaiveDate::parse_from_str(&req.effective_date, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid effective_date format (expected YYYY-MM-DD)")
            })?
        };

        // Parse idempotency key
        let idempotency_key = if req.idempotency_key.is_empty() {
            None
        } else {
            Some(req.idempotency_key.as_str())
        };

        // Parse metadata
        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid metadata JSON")
            })?)
        };

        // Post the transaction
        let (journal_id, inserted_entries, currency) = self
            .db
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
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["PostTransaction", "error"])
                    .inc();
                match e {
                    service_core::error::AppError::BadRequest(err) => {
                        Status::invalid_argument(err.to_string())
                    }
                    _ => Status::internal("Failed to post transaction"),
                }
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["PostTransaction", "ok"])
            .inc();
        TRANSACTIONS_TOTAL.with_label_values(&["ok"]).inc();

        // P3: Record entry and amount metrics
        for entry in &entries {
            let direction_str = entry.direction.as_str();
            ENTRIES_TOTAL.with_label_values(&[direction_str]).inc();
            // Convert Decimal to f64 for counter (counters only accept f64)
            if let Some(amount_f64) = entry.amount.to_f64() {
                AMOUNT_TOTAL
                    .with_label_values(&[direction_str, &currency])
                    .inc_by(amount_f64);
            }
        }

        timer.observe_duration();

        info!(
            journal_id = %journal_id,
            entry_count = inserted_entries.len(),
            "Transaction posted successfully"
        );

        Ok(Response::new(PostTransactionResponse {
            transaction: Some(Self::entries_to_transaction(
                tenant_id,
                journal_id,
                &inserted_entries,
            )),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "GetTransaction")
    )]
    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetTransaction"])
            .start_timer();

        let req = request.into_inner();

        // Parse IDs
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetTransaction", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let journal_id = Uuid::parse_str(&req.journal_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetTransaction", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid journal_id format")
        })?;

        let entries = self
            .db
            .get_entries_by_journal(tenant_id, journal_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get transaction");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetTransaction", "error"])
                    .inc();
                Status::internal("Failed to get transaction")
            })?;

        timer.observe_duration();

        if entries.is_empty() {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetTransaction", "not_found"])
                .inc();
            return Err(Status::not_found("Transaction not found"));
        }

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GetTransaction", "ok"])
            .inc();

        Ok(Response::new(GetTransactionResponse {
            transaction: Some(Self::entries_to_transaction(
                tenant_id, journal_id, &entries,
            )),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "ListTransactions")
    )]
    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["ListTransactions"])
            .start_timer();

        let req = request.into_inner();

        // Parse tenant_id
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["ListTransactions", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        // Parse optional account_id filter
        let account_id = if req.account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.account_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListTransactions", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid account_id format")
            })?)
        };

        // Parse date filters
        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListTransactions", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid start_date format (expected YYYY-MM-DD)")
                })?,
            )
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListTransactions", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid end_date format (expected YYYY-MM-DD)")
                })?,
            )
        };

        // Parse page token
        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListTransactions", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid page_token format")
            })?)
        };

        let page_size = if req.page_size <= 0 {
            20
        } else {
            req.page_size
        };

        let transactions = self
            .db
            .list_transactions(
                tenant_id, account_id, start_date, end_date, page_size, page_token,
            )
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to list transactions");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListTransactions", "error"])
                    .inc();
                Status::internal("Failed to list transactions")
            })?;

        timer.observe_duration();

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["ListTransactions", "ok"])
            .inc();

        // Generate next page token
        let next_page_token = if transactions.len() == page_size as usize {
            transactions.last().map(|(jid, _)| jid.to_string())
        } else {
            None
        };

        Ok(Response::new(ListTransactionsResponse {
            transactions: transactions
                .iter()
                .map(|(jid, entries)| Self::entries_to_transaction(tenant_id, *jid, entries))
                .collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "GetBalance")
    )]
    async fn get_balance(
        &self,
        request: Request<GetBalanceRequest>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetBalance"])
            .start_timer();

        let req = request.into_inner();

        // Parse IDs
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetBalance", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetBalance", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid account_id format")
        })?;

        // Parse as_of_date
        let as_of_date = if req.as_of_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["GetBalance", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid as_of_date format (expected YYYY-MM-DD)")
                })?,
            )
        };

        let result = self
            .db
            .get_balance(tenant_id, account_id, as_of_date)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get balance");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetBalance", "error"])
                    .inc();
                Status::internal("Failed to get balance")
            })?;

        timer.observe_duration();

        match result {
            Some((balance, currency)) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetBalance", "ok"])
                    .inc();
                Ok(Response::new(GetBalanceResponse {
                    account_id: account_id.to_string(),
                    balance: balance.to_string(),
                    currency,
                    as_of_date: as_of_date
                        .unwrap_or_else(|| chrono::Utc::now().date_naive())
                        .to_string(),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetBalance", "not_found"])
                    .inc();
                Err(Status::not_found("Account not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "GetBalances")
    )]
    async fn get_balances(
        &self,
        request: Request<GetBalancesRequest>,
    ) -> Result<Response<GetBalancesResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetBalances"])
            .start_timer();

        let req = request.into_inner();

        // Parse tenant_id
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetBalances", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        // Parse account_ids
        let mut account_ids = Vec::with_capacity(req.account_ids.len());
        for id in &req.account_ids {
            let account_id = Uuid::parse_str(id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetBalances", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid account_id format")
            })?;
            account_ids.push(account_id);
        }

        // Parse as_of_date
        let as_of_date = if req.as_of_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["GetBalances", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid as_of_date format (expected YYYY-MM-DD)")
                })?,
            )
        };

        let results = self
            .db
            .get_balances(tenant_id, &account_ids, as_of_date)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get balances");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetBalances", "error"])
                    .inc();
                Status::internal("Failed to get balances")
            })?;

        timer.observe_duration();

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GetBalances", "ok"])
            .inc();

        let as_of_str = as_of_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive())
            .to_string();

        Ok(Response::new(GetBalancesResponse {
            balances: results
                .iter()
                .map(|(account_id, balance, currency)| GetBalanceResponse {
                    account_id: account_id.to_string(),
                    balance: balance.to_string(),
                    currency: currency.clone(),
                    as_of_date: as_of_str.clone(),
                })
                .collect(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "ledger-service", method = "GetStatement")
    )]
    async fn get_statement(
        &self,
        request: Request<GetStatementRequest>,
    ) -> Result<Response<GetStatementResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetStatement"])
            .start_timer();

        let req = request.into_inner();

        // Parse IDs
        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetStatement", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let account_id = Uuid::parse_str(&req.account_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetStatement", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid account_id format")
        })?;

        // Parse dates
        let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetStatement", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid start_date format (expected YYYY-MM-DD)")
        })?;

        let end_date = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetStatement", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid end_date format (expected YYYY-MM-DD)")
        })?;

        if end_date < start_date {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetStatement", "invalid_argument"])
                .inc();
            return Err(Status::invalid_argument("end_date must be >= start_date"));
        }

        let result = self
            .db
            .get_statement(tenant_id, account_id, start_date, end_date)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get statement");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetStatement", "error"])
                    .inc();
                Status::internal("Failed to get statement")
            })?;

        timer.observe_duration();

        match result {
            Some((currency, opening_balance, closing_balance, entries)) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetStatement", "ok"])
                    .inc();

                // Build statement lines with running balance
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
                            amount: e.amount.to_string(),
                            running_balance: running.to_string(),
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
                    opening_balance: opening_balance.to_string(),
                    closing_balance: closing_balance.to_string(),
                    lines,
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetStatement", "not_found"])
                    .inc();
                Err(Status::not_found("Account not found"))
            }
        }
    }
}
