//! gRPC service implementation for ReconciliationService.

use crate::grpc::capability_check::{capabilities, CapabilityChecker};
use crate::grpc::proto::*;
use crate::services::{
    record_error, record_reconciliation_operation, record_statement_import,
    record_transaction_match, Database,
};
use service_core::grpc::proto::ledger::AccountType as LedgerAccountType;
use service_core::grpc::LedgerClient;
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// ReconciliationService gRPC implementation.
pub struct ReconciliationServiceImpl {
    db: Arc<Database>,
    capability_checker: Arc<CapabilityChecker>,
    ledger_client: Option<Arc<LedgerClient>>,
}

impl ReconciliationServiceImpl {
    pub fn new(
        db: Arc<Database>,
        capability_checker: Arc<CapabilityChecker>,
        ledger_client: Option<Arc<LedgerClient>>,
    ) -> Self {
        Self {
            db,
            capability_checker,
            ledger_client,
        }
    }
}

#[tonic::async_trait]
impl reconciliation_service_server::ReconciliationService for ReconciliationServiceImpl {
    // =========================================================================
    // Bank Account Management
    // =========================================================================

    async fn register_bank_account(
        &self,
        request: Request<RegisterBankAccountRequest>,
    ) -> Result<Response<RegisterBankAccountResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_BANK_ACCOUNT_CREATE)
            .await?;

        let req = request.into_inner();
        tracing::info!(
            bank_name = %req.bank_name,
            ledger_account_id = %req.ledger_account_id,
            "Registering bank account"
        );

        // Check for duplicate ledger_account_id
        let existing = self
            .db
            .get_bank_account_by_ledger_id(&_auth.tenant_id, &req.ledger_account_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to check existing account: {}", e)))?;

        if existing.is_some() {
            return Err(Status::already_exists(
                "A bank account with this ledger_account_id already exists",
            ));
        }

        // Validate ledger account exists and is asset type
        if let Some(ref ledger_client) = self.ledger_client {
            let ledger_response = ledger_client
                .get_account(&_auth.tenant_id, &req.ledger_account_id)
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

        let bank_account = self
            .db
            .create_bank_account(
                &_auth.tenant_id,
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

    async fn get_bank_account(
        &self,
        request: Request<GetBankAccountRequest>,
    ) -> Result<Response<GetBankAccountResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_BANK_ACCOUNT_READ)
            .await?;

        let req = request.into_inner();
        let bank_account = self
            .db
            .get_bank_account(&_auth.tenant_id, &req.bank_account_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get bank account: {}", e)))?
            .ok_or_else(|| Status::not_found("Bank account not found"))?;

        Ok(Response::new(GetBankAccountResponse {
            bank_account: Some(bank_account.into()),
        }))
    }

    async fn list_bank_accounts(
        &self,
        request: Request<ListBankAccountsRequest>,
    ) -> Result<Response<ListBankAccountsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_BANK_ACCOUNT_READ)
            .await?;

        let req = request.into_inner();
        let (accounts, next_token) = self
            .db
            .list_bank_accounts(&_auth.tenant_id, req.page_size, req.page_token.as_deref())
            .await
            .map_err(|e| Status::internal(format!("Failed to list bank accounts: {}", e)))?;

        Ok(Response::new(ListBankAccountsResponse {
            bank_accounts: accounts.into_iter().map(|a| a.into()).collect(),
            next_page_token: next_token,
        }))
    }

    async fn update_bank_account(
        &self,
        request: Request<UpdateBankAccountRequest>,
    ) -> Result<Response<UpdateBankAccountResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_BANK_ACCOUNT_UPDATE)
            .await?;

        let req = request.into_inner();
        let bank_account = self
            .db
            .update_bank_account(
                &_auth.tenant_id,
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

    // =========================================================================
    // Statement Import
    // =========================================================================

    async fn import_statement(
        &self,
        request: Request<ImportStatementRequest>,
    ) -> Result<Response<ImportStatementResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_IMPORT)
            .await?;

        let req = request.into_inner();
        tracing::info!(
            bank_account_id = %req.bank_account_id,
            document_id = %req.document_id,
            "Importing statement"
        );

        let statement = self
            .db
            .create_statement(&_auth.tenant_id, &req.bank_account_id, &req.document_id)
            .await
            .map_err(|e| {
                record_statement_import("failed");
                record_error("database_error");
                Status::internal(format!("Failed to import statement: {}", e))
            })?;

        record_statement_import("success");

        // TODO: Trigger async GenAI extraction via genai-service

        Ok(Response::new(ImportStatementResponse {
            statement: Some(statement.into()),
        }))
    }

    async fn get_statement(
        &self,
        request: Request<GetStatementRequest>,
    ) -> Result<Response<GetStatementResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
            .await?;

        let req = request.into_inner();
        let statement = self
            .db
            .get_statement(&_auth.tenant_id, &req.statement_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get statement: {}", e)))?
            .ok_or_else(|| Status::not_found("Statement not found"))?;

        Ok(Response::new(GetStatementResponse {
            statement: Some(statement.into()),
        }))
    }

    async fn list_statements(
        &self,
        request: Request<ListStatementsRequest>,
    ) -> Result<Response<ListStatementsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
            .await?;

        let req = request.into_inner();
        let (statements, next_token) = self
            .db
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

    async fn get_staged_transactions(
        &self,
        request: Request<GetStagedTransactionsRequest>,
    ) -> Result<Response<GetStagedTransactionsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_READ)
            .await?;

        let req = request.into_inner();
        let (transactions, next_token) = self
            .db
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

    async fn update_staged_transaction(
        &self,
        request: Request<UpdateStagedTransactionRequest>,
    ) -> Result<Response<UpdateStagedTransactionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STAGED_UPDATE)
            .await?;

        let req = request.into_inner();
        let transaction = self
            .db
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

    async fn commit_statement(
        &self,
        request: Request<CommitStatementRequest>,
    ) -> Result<Response<CommitStatementResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_COMMIT)
            .await?;

        let req = request.into_inner();
        let (statement, count) = self
            .db
            .commit_statement(&_auth.tenant_id, &req.statement_id)
            .await
            .map_err(|e| {
                record_statement_import("commit_failed");
                record_error("database_error");
                Status::internal(format!("Failed to commit statement: {}", e))
            })?;

        record_statement_import("committed");

        // Apply matching rules to auto-match transactions
        let auto_matched = self
            .db
            .apply_matching_rules(&_auth.tenant_id, &req.statement_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to apply matching rules");
                record_error("auto_match_error");
                0
            });

        // Record auto-matched transactions
        for _ in 0..auto_matched {
            record_transaction_match("auto");
        }

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

    async fn abandon_statement(
        &self,
        request: Request<AbandonStatementRequest>,
    ) -> Result<Response<AbandonStatementResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_STATEMENT_ABANDON)
            .await?;

        let req = request.into_inner();
        self.db
            .abandon_statement(&_auth.tenant_id, &req.statement_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to abandon statement: {}", e)))?;

        Ok(Response::new(AbandonStatementResponse { success: true }))
    }

    // =========================================================================
    // Matching Rules
    // =========================================================================

    async fn create_matching_rule(
        &self,
        request: Request<CreateMatchingRuleRequest>,
    ) -> Result<Response<CreateMatchingRuleResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_RULE_CREATE)
            .await?;

        let req = request.into_inner();

        // Validate name is not empty
        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("Rule name cannot be empty"));
        }

        let match_type = MatchType::try_from(req.match_type)
            .map_err(|_| Status::invalid_argument("Invalid match_type"))?;

        // Validate regex pattern if match_type is regex
        if match_type == MatchType::Regex {
            if let Err(e) = regex::Regex::new(&req.description_pattern) {
                return Err(Status::invalid_argument(format!(
                    "Invalid regex pattern: {}",
                    e
                )));
            }
        }

        let rule = self
            .db
            .create_matching_rule(
                &_auth.tenant_id,
                &req.name,
                &req.description_pattern,
                match_type,
                req.target_account_id.as_deref(),
                req.priority.unwrap_or(0),
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to create matching rule: {}", e)))?;

        Ok(Response::new(CreateMatchingRuleResponse {
            rule: Some(rule.into()),
        }))
    }

    async fn get_matching_rule(
        &self,
        request: Request<GetMatchingRuleRequest>,
    ) -> Result<Response<GetMatchingRuleResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_RULE_READ)
            .await?;

        let req = request.into_inner();
        let rule = self
            .db
            .get_matching_rule(&_auth.tenant_id, &req.rule_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get matching rule: {}", e)))?
            .ok_or_else(|| Status::not_found("Matching rule not found"))?;

        Ok(Response::new(GetMatchingRuleResponse {
            rule: Some(rule.into()),
        }))
    }

    async fn list_matching_rules(
        &self,
        request: Request<ListMatchingRulesRequest>,
    ) -> Result<Response<ListMatchingRulesResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_RULE_READ)
            .await?;

        let req = request.into_inner();
        let (rules, next_token) = self
            .db
            .list_matching_rules(
                &_auth.tenant_id,
                req.page_size,
                req.page_token.as_deref(),
                req.active_only.unwrap_or(false),
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to list matching rules: {}", e)))?;

        Ok(Response::new(ListMatchingRulesResponse {
            rules: rules.into_iter().map(|r| r.into()).collect(),
            next_page_token: next_token,
        }))
    }

    async fn update_matching_rule(
        &self,
        request: Request<UpdateMatchingRuleRequest>,
    ) -> Result<Response<UpdateMatchingRuleResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_RULE_UPDATE)
            .await?;

        let req = request.into_inner();
        let match_type = req
            .match_type
            .map(MatchType::try_from)
            .transpose()
            .map_err(|_| Status::invalid_argument("Invalid match_type"))?;

        // Validate regex pattern if match_type is regex and pattern is provided
        if match_type == Some(MatchType::Regex) {
            if let Some(ref pattern) = req.description_pattern {
                if let Err(e) = regex::Regex::new(pattern) {
                    return Err(Status::invalid_argument(format!(
                        "Invalid regex pattern: {}",
                        e
                    )));
                }
            }
        }

        let rule = self
            .db
            .update_matching_rule(
                &_auth.tenant_id,
                &req.rule_id,
                req.name.as_deref(),
                req.description_pattern.as_deref(),
                match_type,
                req.target_account_id.as_deref(),
                req.priority,
                req.is_active,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to update matching rule: {}", e)))?
            .ok_or_else(|| Status::not_found("Matching rule not found"))?;

        Ok(Response::new(UpdateMatchingRuleResponse {
            rule: Some(rule.into()),
        }))
    }

    async fn delete_matching_rule(
        &self,
        request: Request<DeleteMatchingRuleRequest>,
    ) -> Result<Response<DeleteMatchingRuleResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_RULE_DELETE)
            .await?;

        let req = request.into_inner();
        self.db
            .delete_matching_rule(&_auth.tenant_id, &req.rule_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete matching rule: {}", e)))?;

        Ok(Response::new(DeleteMatchingRuleResponse { success: true }))
    }

    // =========================================================================
    // Transaction Matching
    // =========================================================================

    async fn match_transaction(
        &self,
        request: Request<MatchTransactionRequest>,
    ) -> Result<Response<MatchTransactionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_MATCH_CREATE)
            .await?;

        let req = request.into_inner();

        // Validate at least one ledger entry is provided
        if req.ledger_entry_ids.is_empty() {
            return Err(Status::invalid_argument(
                "At least one ledger_entry_id is required",
            ));
        }

        // Verify the bank transaction exists and belongs to tenant
        let bank_txn = self
            .db
            .get_bank_transaction(&_auth.tenant_id, &req.bank_transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Bank transaction not found"))?;

        // Verify transaction is in matchable status (unmatched)
        if bank_txn.status != "unmatched" {
            return Err(Status::failed_precondition(format!(
                "Transaction cannot be matched: current status is '{}'",
                bank_txn.status
            )));
        }

        tracing::info!(
            bank_transaction_id = %req.bank_transaction_id,
            ledger_entry_count = req.ledger_entry_ids.len(),
            is_split_match = req.ledger_entry_ids.len() > 1,
            "Matching transaction"
        );

        let matches = self
            .db
            .match_transaction(
                &_auth.tenant_id,
                &req.bank_transaction_id,
                &req.ledger_entry_ids,
                "manual",
                &_auth.user_id,
            )
            .await
            .map_err(|e| {
                record_error("match_error");
                Status::internal(format!("Failed to match transaction: {}", e))
            })?;

        record_transaction_match("manual");

        Ok(Response::new(MatchTransactionResponse {
            matches: matches.into_iter().map(|m| m.into()).collect(),
        }))
    }

    async fn unmatch_transaction(
        &self,
        request: Request<UnmatchTransactionRequest>,
    ) -> Result<Response<UnmatchTransactionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_MATCH_DELETE)
            .await?;

        let req = request.into_inner();

        // Verify the bank transaction exists and belongs to tenant
        let bank_txn = self
            .db
            .get_bank_transaction(&_auth.tenant_id, &req.bank_transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Bank transaction not found"))?;

        // Verify transaction is matched (can unmatch matched or manually_matched)
        if bank_txn.status != "matched" && bank_txn.status != "manually_matched" {
            return Err(Status::failed_precondition(format!(
                "Transaction is not matched: current status is '{}'",
                bank_txn.status
            )));
        }

        tracing::info!(
            bank_transaction_id = %req.bank_transaction_id,
            previous_status = %bank_txn.status,
            "Unmatching transaction"
        );

        self.db
            .unmatch_transaction(&_auth.tenant_id, &req.bank_transaction_id)
            .await
            .map_err(|e| {
                record_error("unmatch_error");
                Status::internal(format!("Failed to unmatch transaction: {}", e))
            })?;

        record_transaction_match("unmatch");

        Ok(Response::new(UnmatchTransactionResponse { success: true }))
    }

    async fn exclude_transaction(
        &self,
        request: Request<ExcludeTransactionRequest>,
    ) -> Result<Response<ExcludeTransactionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_EXCLUDE)
            .await?;

        let req = request.into_inner();
        self.db
            .exclude_transaction(
                &_auth.tenant_id,
                &req.bank_transaction_id,
                req.reason.as_deref(),
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to exclude transaction: {}", e)))?;

        Ok(Response::new(ExcludeTransactionResponse { success: true }))
    }

    async fn get_candidate_entries(
        &self,
        request: Request<GetCandidateEntriesRequest>,
    ) -> Result<Response<GetCandidateEntriesResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_TRANSACTION_READ)
            .await?;

        let req = request.into_inner();
        let date_range_days = req.date_range_days.unwrap_or(7) as i64;
        let limit = req.limit.unwrap_or(20) as usize;

        // Get the bank transaction to find its date and amount
        let bank_txn = self
            .db
            .get_bank_transaction(&_auth.tenant_id, &req.bank_transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Bank transaction not found"))?;

        // Get the bank account to find the linked ledger account
        let statement = self
            .db
            .get_statement_by_transaction(&_auth.tenant_id, &req.bank_transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get statement: {}", e)))?
            .ok_or_else(|| Status::internal("Statement not found for transaction"))?;

        let bank_account = self
            .db
            .get_bank_account(&_auth.tenant_id, &statement.bank_account_id.to_string())
            .await
            .map_err(|e| Status::internal(format!("Failed to get bank account: {}", e)))?
            .ok_or_else(|| Status::internal("Bank account not found"))?;

        tracing::info!(
            bank_transaction_id = %req.bank_transaction_id,
            ledger_account_id = %bank_account.ledger_account_id,
            date_range_days = date_range_days,
            "Getting candidate entries from ledger"
        );

        // Query ledger-service for transactions if client is available
        let Some(ref ledger_client) = self.ledger_client else {
            tracing::warn!("Ledger client not configured, returning empty candidates");
            return Ok(Response::new(GetCandidateEntriesResponse {
                candidates: vec![],
            }));
        };

        // Calculate date range centered on transaction date
        let start_date = bank_txn
            .transaction_date
            .checked_sub_signed(chrono::Duration::days(date_range_days))
            .unwrap_or(bank_txn.transaction_date);
        let end_date = bank_txn
            .transaction_date
            .checked_add_signed(chrono::Duration::days(date_range_days))
            .unwrap_or(bank_txn.transaction_date);

        // Query ledger transactions for this account
        let ledger_response = ledger_client
            .list_transactions(
                &_auth.tenant_id,
                Some(&bank_account.ledger_account_id.to_string()),
                Some(&start_date.format("%Y-%m-%d").to_string()),
                Some(&end_date.format("%Y-%m-%d").to_string()),
                100, // Fetch more than limit to allow filtering
                None,
            )
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Failed to query ledger transactions");
                Status::internal("Failed to query ledger transactions")
            })?;

        // Convert to candidates with match likelihood scores
        let bank_amount = bank_txn.amount;
        let bank_date = bank_txn.transaction_date;

        let mut candidates: Vec<CandidateEntry> = ledger_response
            .transactions
            .into_iter()
            .flat_map(|txn| {
                txn.entries.into_iter().filter_map(|entry| {
                    // Only include entries for our account
                    if entry.account_id != bank_account.ledger_account_id.to_string() {
                        return None;
                    }

                    // Parse entry amount
                    let entry_amount: rust_decimal::Decimal = entry.amount.parse().ok()?;

                    // Calculate signed amount based on direction
                    // For asset accounts: debit increases, credit decreases
                    let signed_amount = if entry.direction == 1 {
                        // Debit
                        entry_amount
                    } else {
                        // Credit
                        -entry_amount
                    };

                    // Calculate amount similarity (0-1)
                    let amount_diff = (bank_amount - signed_amount).abs();
                    let max_amount = bank_amount.abs().max(signed_amount.abs());
                    let amount_score = if max_amount.is_zero() {
                        1.0
                    } else {
                        let ratio =
                            rust_decimal::prelude::ToPrimitive::to_f64(&(amount_diff / max_amount))
                                .unwrap_or(1.0);
                        (1.0 - ratio).max(0.0)
                    };

                    // Calculate date proximity score (0-1)
                    let entry_date =
                        chrono::NaiveDate::parse_from_str(&entry.effective_date, "%Y-%m-%d")
                            .ok()?;
                    let days_diff = (bank_date - entry_date).num_days().abs() as f64;
                    let date_score = (1.0 - days_diff / (date_range_days as f64 * 2.0)).max(0.0);

                    // Combined score (weighted: 70% amount, 30% date)
                    let match_likelihood = amount_score * 0.7 + date_score * 0.3;

                    Some(CandidateEntry {
                        ledger_entry_id: entry.entry_id,
                        date: entry.effective_date,
                        description: entry.metadata.clone(),
                        amount: if signed_amount.is_sign_negative() {
                            format!("-{}", entry.amount)
                        } else {
                            entry.amount
                        },
                        account_name: bank_account.bank_name.clone(),
                        match_likelihood,
                    })
                })
            })
            .collect();

        // Sort by likelihood descending and limit results
        candidates.sort_by(|a, b| {
            b.match_likelihood
                .partial_cmp(&a.match_likelihood)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(limit);

        Ok(Response::new(GetCandidateEntriesResponse { candidates }))
    }

    // =========================================================================
    // AI Matching
    // =========================================================================

    async fn get_ai_suggestions(
        &self,
        request: Request<GetAiSuggestionsRequest>,
    ) -> Result<Response<GetAiSuggestionsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_AI_SUGGEST)
            .await?;

        // TODO: Integrate with genai-service for AI suggestions
        let _req = request.into_inner();

        Ok(Response::new(GetAiSuggestionsResponse {
            suggestions: vec![],
        }))
    }

    async fn confirm_suggestion(
        &self,
        request: Request<ConfirmSuggestionRequest>,
    ) -> Result<Response<ConfirmSuggestionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_AI_CONFIRM)
            .await?;

        // TODO: Implement suggestion confirmation
        let _req = request.into_inner();

        Err(Status::unimplemented("AI suggestions not yet implemented"))
    }

    async fn reject_suggestion(
        &self,
        request: Request<RejectSuggestionRequest>,
    ) -> Result<Response<RejectSuggestionResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_AI_CONFIRM)
            .await?;

        // TODO: Implement suggestion rejection
        let _req = request.into_inner();

        Err(Status::unimplemented("AI suggestions not yet implemented"))
    }

    // =========================================================================
    // Reconciliation Process
    // =========================================================================

    async fn start_reconciliation(
        &self,
        request: Request<StartReconciliationRequest>,
    ) -> Result<Response<StartReconciliationResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_START)
            .await?;

        let req = request.into_inner();

        // Validate bank account exists and belongs to tenant
        let bank_account = self
            .db
            .get_bank_account(&_auth.tenant_id, &req.bank_account_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get bank account: {}", e)))?
            .ok_or_else(|| Status::not_found("Bank account not found"))?;

        // Get expected balance from ledger if client is available
        let expected_balance = if let Some(ref ledger_client) = self.ledger_client {
            match ledger_client
                .get_balance(
                    &_auth.tenant_id,
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

        let reconciliation = self
            .db
            .start_reconciliation(
                &_auth.tenant_id,
                &req.bank_account_id,
                &req.period_start,
                &req.period_end,
                expected_balance.as_deref(),
            )
            .await
            .map_err(|e| {
                record_reconciliation_operation("start", "failed");
                record_error("database_error");
                Status::internal(format!("Failed to start reconciliation: {}", e))
            })?;

        record_reconciliation_operation("start", "success");

        Ok(Response::new(StartReconciliationResponse {
            reconciliation: Some(reconciliation.into()),
        }))
    }

    async fn get_reconciliation(
        &self,
        request: Request<GetReconciliationRequest>,
    ) -> Result<Response<GetReconciliationResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_READ)
            .await?;

        let req = request.into_inner();
        let reconciliation = self
            .db
            .get_reconciliation(&_auth.tenant_id, &req.reconciliation_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get reconciliation: {}", e)))?
            .ok_or_else(|| Status::not_found("Reconciliation not found"))?;

        Ok(Response::new(GetReconciliationResponse {
            reconciliation: Some(reconciliation.into()),
        }))
    }

    async fn list_reconciliations(
        &self,
        request: Request<ListReconciliationsRequest>,
    ) -> Result<Response<ListReconciliationsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_READ)
            .await?;

        let req = request.into_inner();
        let (reconciliations, next_token) = self
            .db
            .list_reconciliations(
                &_auth.tenant_id,
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

    async fn complete_reconciliation(
        &self,
        request: Request<CompleteReconciliationRequest>,
    ) -> Result<Response<CompleteReconciliationResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_COMPLETE)
            .await?;

        let req = request.into_inner();
        let reconciliation = self
            .db
            .complete_reconciliation(&_auth.tenant_id, &req.reconciliation_id)
            .await
            .map_err(|e| {
                record_reconciliation_operation("complete", "failed");
                record_error("database_error");
                Status::internal(format!("Failed to complete reconciliation: {}", e))
            })?;

        record_reconciliation_operation("complete", "success");

        Ok(Response::new(CompleteReconciliationResponse {
            reconciliation: Some(reconciliation.into()),
        }))
    }

    async fn abandon_reconciliation(
        &self,
        request: Request<AbandonReconciliationRequest>,
    ) -> Result<Response<AbandonReconciliationResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_ABANDON)
            .await?;

        let req = request.into_inner();
        self.db
            .abandon_reconciliation(&_auth.tenant_id, &req.reconciliation_id)
            .await
            .map_err(|e| {
                record_reconciliation_operation("abandon", "failed");
                record_error("database_error");
                Status::internal(format!("Failed to abandon reconciliation: {}", e))
            })?;

        record_reconciliation_operation("abandon", "success");

        Ok(Response::new(AbandonReconciliationResponse {
            success: true,
        }))
    }

    // =========================================================================
    // Adjustments
    // =========================================================================

    async fn create_adjustment(
        &self,
        request: Request<CreateAdjustmentRequest>,
    ) -> Result<Response<CreateAdjustmentResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_ADJUSTMENT_CREATE)
            .await?;

        let req = request.into_inner();

        // Validate adjustment type
        let adjustment_type = AdjustmentType::try_from(req.adjustment_type)
            .map_err(|_| Status::invalid_argument("Invalid adjustment_type"))?;

        // Validate description is not empty
        if req.description.trim().is_empty() {
            return Err(Status::invalid_argument("Description cannot be empty"));
        }

        // Validate reconciliation exists and is in progress
        let reconciliation = self
            .db
            .get_reconciliation(&_auth.tenant_id, &req.reconciliation_id)
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

        let adjustment = self
            .db
            .create_adjustment(
                &_auth.tenant_id,
                &req.reconciliation_id,
                adjustment_type,
                &req.description,
                &req.amount,
            )
            .await
            .map_err(|e| {
                record_reconciliation_operation("adjustment", "failed");
                record_error("database_error");
                Status::internal(format!("Failed to create adjustment: {}", e))
            })?;

        record_reconciliation_operation("adjustment", "success");

        Ok(Response::new(CreateAdjustmentResponse {
            adjustment: Some(adjustment.into()),
        }))
    }

    async fn list_adjustments(
        &self,
        request: Request<ListAdjustmentsRequest>,
    ) -> Result<Response<ListAdjustmentsResponse>, Status> {
        let _auth = self
            .capability_checker
            .require_capability(&request, capabilities::RECONCILIATION_ADJUSTMENT_READ)
            .await?;

        let req = request.into_inner();
        let (adjustments, next_token) = self
            .db
            .list_adjustments(
                &_auth.tenant_id,
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
}
