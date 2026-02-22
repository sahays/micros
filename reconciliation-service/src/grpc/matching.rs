//! Matching rules and transaction matching gRPC handlers.

use crate::grpc::proto::*;
use crate::services::Database;
use service_core::grpc::LedgerClient;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub async fn create_matching_rule(
    db: &Arc<Database>,
    request: Request<CreateMatchingRuleRequest>,
) -> Result<Response<CreateMatchingRuleResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

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

    let rule = db
        .create_matching_rule(
            &app_id,
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

pub async fn get_matching_rule(
    db: &Arc<Database>,
    request: Request<GetMatchingRuleRequest>,
) -> Result<Response<GetMatchingRuleResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let rule = db
        .get_matching_rule(&app_id, &req.rule_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get matching rule: {}", e)))?
        .ok_or_else(|| Status::not_found("Matching rule not found"))?;

    Ok(Response::new(GetMatchingRuleResponse {
        rule: Some(rule.into()),
    }))
}

pub async fn list_matching_rules(
    db: &Arc<Database>,
    request: Request<ListMatchingRulesRequest>,
) -> Result<Response<ListMatchingRulesResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let (rules, next_token) = db
        .list_matching_rules(
            &app_id,
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

pub async fn update_matching_rule(
    db: &Arc<Database>,
    request: Request<UpdateMatchingRuleRequest>,
) -> Result<Response<UpdateMatchingRuleResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

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

    let rule = db
        .update_matching_rule(
            &app_id,
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

pub async fn delete_matching_rule(
    db: &Arc<Database>,
    request: Request<DeleteMatchingRuleRequest>,
) -> Result<Response<DeleteMatchingRuleResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    db.delete_matching_rule(&app_id, &req.rule_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to delete matching rule: {}", e)))?;

    Ok(Response::new(DeleteMatchingRuleResponse { success: true }))
}

pub async fn match_transaction(
    db: &Arc<Database>,
    request: Request<MatchTransactionRequest>,
) -> Result<Response<MatchTransactionResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;
    let user_id = service_core::grpc::extract_user_id(&request)
        .unwrap_or_default();

    let req = request.into_inner();

    // Validate at least one ledger entry is provided
    if req.ledger_entry_ids.is_empty() {
        return Err(Status::invalid_argument(
            "At least one ledger_entry_id is required",
        ));
    }

    // Verify the bank transaction exists and belongs to tenant
    let bank_txn = db
        .get_bank_transaction(&app_id, &req.bank_transaction_id)
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

    let matches = db
        .match_transaction(
            &app_id,
            &req.bank_transaction_id,
            &req.ledger_entry_ids,
            "manual",
            &user_id,
        )
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to match transaction: {}", e))
        })?;

    Ok(Response::new(MatchTransactionResponse {
        matches: matches.into_iter().map(|m| m.into()).collect(),
    }))
}

pub async fn unmatch_transaction(
    db: &Arc<Database>,
    request: Request<UnmatchTransactionRequest>,
) -> Result<Response<UnmatchTransactionResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();

    // Verify the bank transaction exists and belongs to tenant
    let bank_txn = db
        .get_bank_transaction(&app_id, &req.bank_transaction_id)
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

    db.unmatch_transaction(&app_id, &req.bank_transaction_id)
        .await
        .map_err(|e| {
            Status::internal(format!("Failed to unmatch transaction: {}", e))
        })?;

    Ok(Response::new(UnmatchTransactionResponse { success: true }))
}

pub async fn exclude_transaction(
    db: &Arc<Database>,
    request: Request<ExcludeTransactionRequest>,
) -> Result<Response<ExcludeTransactionResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    db.exclude_transaction(
        &app_id,
        &req.bank_transaction_id,
        req.reason.as_deref(),
    )
    .await
    .map_err(|e| Status::internal(format!("Failed to exclude transaction: {}", e)))?;

    Ok(Response::new(ExcludeTransactionResponse { success: true }))
}

#[allow(clippy::too_many_arguments)]
pub async fn get_candidate_entries(
    db: &Arc<Database>,
    ledger_client: &Option<Arc<LedgerClient>>,
    request: Request<GetCandidateEntriesRequest>,
) -> Result<Response<GetCandidateEntriesResponse>, Status> {
    let app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    let req = request.into_inner();
    let date_range_days = req.date_range_days.unwrap_or(7) as i64;
    let limit = req.limit.unwrap_or(20) as usize;

    // Get the bank transaction to find its date and amount
    let bank_txn = db
        .get_bank_transaction(&app_id, &req.bank_transaction_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?
        .ok_or_else(|| Status::not_found("Bank transaction not found"))?;

    // Get the bank account to find the linked ledger account
    let statement = db
        .get_statement_by_transaction(&app_id, &req.bank_transaction_id)
        .await
        .map_err(|e| Status::internal(format!("Failed to get statement: {}", e)))?
        .ok_or_else(|| Status::internal("Statement not found for transaction"))?;

    let bank_account = db
        .get_bank_account(&app_id, &statement.bank_account_id.to_string())
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
    let Some(ref ledger_client) = ledger_client else {
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
            &app_id,
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
                    chrono::NaiveDate::parse_from_str(&entry.effective_date, "%Y-%m-%d").ok()?;
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

pub async fn get_ai_suggestions(
    request: Request<GetAiSuggestionsRequest>,
) -> Result<Response<GetAiSuggestionsResponse>, Status> {
    let _app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    // TODO: Integrate with genai-service for AI suggestions
    let _req = request.into_inner();

    Ok(Response::new(GetAiSuggestionsResponse {
        suggestions: vec![],
    }))
}

pub async fn confirm_suggestion(
    request: Request<ConfirmSuggestionRequest>,
) -> Result<Response<ConfirmSuggestionResponse>, Status> {
    let _app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    // TODO: Implement suggestion confirmation
    let _req = request.into_inner();

    Err(Status::unimplemented("AI suggestions not yet implemented"))
}

pub async fn reject_suggestion(
    request: Request<RejectSuggestionRequest>,
) -> Result<Response<RejectSuggestionResponse>, Status> {
    let _app_id = service_core::grpc::extract_app_id(&request)
        .ok_or_else(|| tonic::Status::unauthenticated("Missing x-app-id header"))?;

    // TODO: Implement suggestion rejection
    let _req = request.into_inner();

    Err(Status::unimplemented("AI suggestions not yet implemented"))
}
