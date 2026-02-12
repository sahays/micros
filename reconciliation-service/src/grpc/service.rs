//! gRPC service implementation for ReconciliationService.

use crate::grpc::capability_check::CapabilityChecker;
use crate::grpc::proto::*;
use crate::grpc::{bank_accounts, matching, reconciliation, statements};
use crate::services::Database;
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
        bank_accounts::register_bank_account(
            &self.db,
            &self.capability_checker,
            &self.ledger_client,
            request,
        )
        .await
    }

    async fn get_bank_account(
        &self,
        request: Request<GetBankAccountRequest>,
    ) -> Result<Response<GetBankAccountResponse>, Status> {
        bank_accounts::get_bank_account(&self.db, &self.capability_checker, request).await
    }

    async fn list_bank_accounts(
        &self,
        request: Request<ListBankAccountsRequest>,
    ) -> Result<Response<ListBankAccountsResponse>, Status> {
        bank_accounts::list_bank_accounts(&self.db, &self.capability_checker, request).await
    }

    async fn update_bank_account(
        &self,
        request: Request<UpdateBankAccountRequest>,
    ) -> Result<Response<UpdateBankAccountResponse>, Status> {
        bank_accounts::update_bank_account(&self.db, &self.capability_checker, request).await
    }

    // =========================================================================
    // Statement Import
    // =========================================================================

    async fn import_statement(
        &self,
        request: Request<ImportStatementRequest>,
    ) -> Result<Response<ImportStatementResponse>, Status> {
        statements::import_statement(&self.db, &self.capability_checker, request).await
    }

    async fn get_statement(
        &self,
        request: Request<GetStatementRequest>,
    ) -> Result<Response<GetStatementResponse>, Status> {
        statements::get_statement(&self.db, &self.capability_checker, request).await
    }

    async fn list_statements(
        &self,
        request: Request<ListStatementsRequest>,
    ) -> Result<Response<ListStatementsResponse>, Status> {
        statements::list_statements(&self.db, &self.capability_checker, request).await
    }

    async fn get_staged_transactions(
        &self,
        request: Request<GetStagedTransactionsRequest>,
    ) -> Result<Response<GetStagedTransactionsResponse>, Status> {
        statements::get_staged_transactions(&self.db, &self.capability_checker, request).await
    }

    async fn update_staged_transaction(
        &self,
        request: Request<UpdateStagedTransactionRequest>,
    ) -> Result<Response<UpdateStagedTransactionResponse>, Status> {
        statements::update_staged_transaction(&self.db, &self.capability_checker, request).await
    }

    async fn commit_statement(
        &self,
        request: Request<CommitStatementRequest>,
    ) -> Result<Response<CommitStatementResponse>, Status> {
        statements::commit_statement(&self.db, &self.capability_checker, request).await
    }

    async fn abandon_statement(
        &self,
        request: Request<AbandonStatementRequest>,
    ) -> Result<Response<AbandonStatementResponse>, Status> {
        statements::abandon_statement(&self.db, &self.capability_checker, request).await
    }

    // =========================================================================
    // Matching Rules
    // =========================================================================

    async fn create_matching_rule(
        &self,
        request: Request<CreateMatchingRuleRequest>,
    ) -> Result<Response<CreateMatchingRuleResponse>, Status> {
        matching::create_matching_rule(&self.db, &self.capability_checker, request).await
    }

    async fn get_matching_rule(
        &self,
        request: Request<GetMatchingRuleRequest>,
    ) -> Result<Response<GetMatchingRuleResponse>, Status> {
        matching::get_matching_rule(&self.db, &self.capability_checker, request).await
    }

    async fn list_matching_rules(
        &self,
        request: Request<ListMatchingRulesRequest>,
    ) -> Result<Response<ListMatchingRulesResponse>, Status> {
        matching::list_matching_rules(&self.db, &self.capability_checker, request).await
    }

    async fn update_matching_rule(
        &self,
        request: Request<UpdateMatchingRuleRequest>,
    ) -> Result<Response<UpdateMatchingRuleResponse>, Status> {
        matching::update_matching_rule(&self.db, &self.capability_checker, request).await
    }

    async fn delete_matching_rule(
        &self,
        request: Request<DeleteMatchingRuleRequest>,
    ) -> Result<Response<DeleteMatchingRuleResponse>, Status> {
        matching::delete_matching_rule(&self.db, &self.capability_checker, request).await
    }

    // =========================================================================
    // Transaction Matching
    // =========================================================================

    async fn match_transaction(
        &self,
        request: Request<MatchTransactionRequest>,
    ) -> Result<Response<MatchTransactionResponse>, Status> {
        matching::match_transaction(&self.db, &self.capability_checker, request).await
    }

    async fn unmatch_transaction(
        &self,
        request: Request<UnmatchTransactionRequest>,
    ) -> Result<Response<UnmatchTransactionResponse>, Status> {
        matching::unmatch_transaction(&self.db, &self.capability_checker, request).await
    }

    async fn exclude_transaction(
        &self,
        request: Request<ExcludeTransactionRequest>,
    ) -> Result<Response<ExcludeTransactionResponse>, Status> {
        matching::exclude_transaction(&self.db, &self.capability_checker, request).await
    }

    async fn get_candidate_entries(
        &self,
        request: Request<GetCandidateEntriesRequest>,
    ) -> Result<Response<GetCandidateEntriesResponse>, Status> {
        matching::get_candidate_entries(
            &self.db,
            &self.capability_checker,
            &self.ledger_client,
            request,
        )
        .await
    }

    // =========================================================================
    // AI Matching
    // =========================================================================

    async fn get_ai_suggestions(
        &self,
        request: Request<GetAiSuggestionsRequest>,
    ) -> Result<Response<GetAiSuggestionsResponse>, Status> {
        matching::get_ai_suggestions(&self.capability_checker, request).await
    }

    async fn confirm_suggestion(
        &self,
        request: Request<ConfirmSuggestionRequest>,
    ) -> Result<Response<ConfirmSuggestionResponse>, Status> {
        matching::confirm_suggestion(&self.capability_checker, request).await
    }

    async fn reject_suggestion(
        &self,
        request: Request<RejectSuggestionRequest>,
    ) -> Result<Response<RejectSuggestionResponse>, Status> {
        matching::reject_suggestion(&self.capability_checker, request).await
    }

    // =========================================================================
    // Reconciliation Process
    // =========================================================================

    async fn start_reconciliation(
        &self,
        request: Request<StartReconciliationRequest>,
    ) -> Result<Response<StartReconciliationResponse>, Status> {
        reconciliation::start_reconciliation(
            &self.db,
            &self.capability_checker,
            &self.ledger_client,
            request,
        )
        .await
    }

    async fn get_reconciliation(
        &self,
        request: Request<GetReconciliationRequest>,
    ) -> Result<Response<GetReconciliationResponse>, Status> {
        reconciliation::get_reconciliation(&self.db, &self.capability_checker, request).await
    }

    async fn list_reconciliations(
        &self,
        request: Request<ListReconciliationsRequest>,
    ) -> Result<Response<ListReconciliationsResponse>, Status> {
        reconciliation::list_reconciliations(&self.db, &self.capability_checker, request).await
    }

    async fn complete_reconciliation(
        &self,
        request: Request<CompleteReconciliationRequest>,
    ) -> Result<Response<CompleteReconciliationResponse>, Status> {
        reconciliation::complete_reconciliation(&self.db, &self.capability_checker, request).await
    }

    async fn abandon_reconciliation(
        &self,
        request: Request<AbandonReconciliationRequest>,
    ) -> Result<Response<AbandonReconciliationResponse>, Status> {
        reconciliation::abandon_reconciliation(&self.db, &self.capability_checker, request).await
    }

    // =========================================================================
    // Adjustments
    // =========================================================================

    async fn create_adjustment(
        &self,
        request: Request<CreateAdjustmentRequest>,
    ) -> Result<Response<CreateAdjustmentResponse>, Status> {
        reconciliation::create_adjustment(&self.db, &self.capability_checker, request).await
    }

    async fn list_adjustments(
        &self,
        request: Request<ListAdjustmentsRequest>,
    ) -> Result<Response<ListAdjustmentsResponse>, Status> {
        reconciliation::list_adjustments(&self.db, &self.capability_checker, request).await
    }
}
