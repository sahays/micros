//! Matching rule database operations for reconciliation-service.

#![allow(clippy::too_many_arguments)]

use crate::grpc::proto;
use crate::models::{BankTransaction, MatchType, MatchingRule, TransactionStatus};
use crate::services::database::Database;
use crate::services::metrics::DB_QUERY_DURATION;
use service_core::error::AppError;
use std::str::FromStr;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Matching Rule Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn create_matching_rule(
        &self,
        tenant_id: &str,
        name: &str,
        description_pattern: &str,
        match_type: proto::MatchType,
        target_account_id: Option<&str>,
        priority: i32,
    ) -> Result<MatchingRule, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_matching_rule"])
            .start_timer();

        let rule_id = Uuid::new_v4();
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let target_uuid: Option<Uuid> = target_account_id
            .map(Uuid::from_str)
            .transpose()
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid target_account_id")))?;

        let rule = sqlx::query_as::<_, MatchingRule>(
            r#"
            INSERT INTO matching_rules (rule_id, tenant_id, name, description_pattern, match_type, target_account_id, priority)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING rule_id, tenant_id, name, description_pattern, match_type, target_account_id, priority, is_active, created_utc
            "#,
        )
        .bind(rule_id)
        .bind(tenant_uuid)
        .bind(name)
        .bind(description_pattern)
        .bind(MatchType::from_proto(match_type).as_str())
        .bind(target_uuid)
        .bind(priority)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create matching rule: {}", e)))?;

        timer.observe_duration();
        info!(rule_id = %rule.rule_id, "Matching rule created");

        Ok(rule)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, rule_id = %rule_id))]
    pub async fn get_matching_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
    ) -> Result<Option<MatchingRule>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_matching_rule"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let rule_uuid = Uuid::from_str(rule_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid rule_id")))?;

        let rule = sqlx::query_as::<_, MatchingRule>(
            r#"
            SELECT rule_id, tenant_id, name, description_pattern, match_type,
                   target_account_id, priority, is_active, created_utc
            FROM matching_rules
            WHERE tenant_id = $1 AND rule_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(rule_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get matching rule: {}", e))
        })?;

        timer.observe_duration();
        Ok(rule)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn list_matching_rules(
        &self,
        tenant_id: &str,
        page_size: i32,
        page_token: Option<&str>,
        active_only: bool,
    ) -> Result<(Vec<MatchingRule>, Option<String>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_matching_rules"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let rules = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, MatchingRule>(
                r#"
                SELECT rule_id, tenant_id, name, description_pattern, match_type, target_account_id, priority, is_active, created_utc
                FROM matching_rules
                WHERE tenant_id = $1 AND rule_id > $2 AND ($3 = FALSE OR is_active = TRUE)
                ORDER BY priority, rule_id
                LIMIT $4
                "#,
            )
            .bind(tenant_uuid)
            .bind(cursor_uuid)
            .bind(active_only)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, MatchingRule>(
                r#"
                SELECT rule_id, tenant_id, name, description_pattern, match_type, target_account_id, priority, is_active, created_utc
                FROM matching_rules
                WHERE tenant_id = $1 AND ($2 = FALSE OR is_active = TRUE)
                ORDER BY priority, rule_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(active_only)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list matching rules: {}", e)))?;

        timer.observe_duration();

        let has_more = rules.len() > limit as usize;
        let mut rules = rules;
        if has_more {
            rules.pop();
        }
        let next_token = if has_more {
            rules.last().map(|r| r.rule_id.to_string())
        } else {
            None
        };

        Ok((rules, next_token))
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, rule_id = %rule_id))]
    pub async fn update_matching_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
        name: Option<&str>,
        description_pattern: Option<&str>,
        match_type: Option<proto::MatchType>,
        target_account_id: Option<&str>,
        priority: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<Option<MatchingRule>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_matching_rule"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let rule_uuid = Uuid::from_str(rule_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid rule_id")))?;
        let target_uuid: Option<Uuid> = target_account_id
            .map(Uuid::from_str)
            .transpose()
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid target_account_id")))?;
        let match_type_str: Option<String> =
            match_type.map(|m| MatchType::from_proto(m).as_str().to_string());

        let rule = sqlx::query_as::<_, MatchingRule>(
            r#"
            UPDATE matching_rules
            SET name = COALESCE($3, name),
                description_pattern = COALESCE($4, description_pattern),
                match_type = COALESCE($5, match_type),
                target_account_id = COALESCE($6, target_account_id),
                priority = COALESCE($7, priority),
                is_active = COALESCE($8, is_active)
            WHERE tenant_id = $1 AND rule_id = $2
            RETURNING rule_id, tenant_id, name, description_pattern, match_type, target_account_id, priority, is_active, created_utc
            "#,
        )
        .bind(tenant_uuid)
        .bind(rule_uuid)
        .bind(name)
        .bind(description_pattern)
        .bind(match_type_str)
        .bind(target_uuid)
        .bind(priority)
        .bind(is_active)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update matching rule: {}", e)))?;

        timer.observe_duration();

        Ok(rule)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, rule_id = %rule_id))]
    pub async fn delete_matching_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
    ) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["delete_matching_rule"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let rule_uuid = Uuid::from_str(rule_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid rule_id")))?;

        sqlx::query(
            r#"
            DELETE FROM matching_rules
            WHERE tenant_id = $1 AND rule_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(rule_uuid)
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to delete matching rule: {}", e))
        })?;

        timer.observe_duration();

        Ok(())
    }

    /// Apply matching rules to unmatched transactions for a statement.
    /// Returns the number of transactions auto-matched.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, statement_id = %statement_id))]
    pub async fn apply_matching_rules(
        &self,
        tenant_id: &str,
        statement_id: &str,
    ) -> Result<i32, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["apply_matching_rules"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        // Get active rules ordered by priority
        let rules = sqlx::query_as::<_, MatchingRule>(
            r#"
            SELECT rule_id, tenant_id, name, description_pattern, match_type,
                   target_account_id, priority, is_active, created_utc
            FROM matching_rules
            WHERE tenant_id = $1 AND is_active = TRUE
            ORDER BY priority, rule_id
            "#,
        )
        .bind(tenant_uuid)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get rules: {}", e)))?;

        if rules.is_empty() {
            timer.observe_duration();
            return Ok(0);
        }

        // Compile regex patterns once
        let compiled_rules: Vec<(MatchingRule, Option<regex::Regex>)> = rules
            .into_iter()
            .map(|rule| {
                let regex = if rule.match_type == MatchType::Regex.as_str() {
                    regex::Regex::new(&rule.description_pattern).ok()
                } else {
                    None
                };
                (rule, regex)
            })
            .collect();

        // Get unmatched transactions for this statement
        let transactions = sqlx::query_as::<_, BankTransaction>(
            r#"
            SELECT transaction_id, statement_id, tenant_id, transaction_date, description,
                   reference, amount, running_balance, status, extraction_confidence,
                   is_modified, created_utc
            FROM bank_transactions
            WHERE statement_id = $1 AND status = 'unmatched'
            "#,
        )
        .bind(stmt_uuid)
        .fetch_all(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get transactions: {}", e))
        })?;

        let mut matched_count = 0;

        for txn in &transactions {
            let desc_lower = txn.description.to_lowercase();

            // Check each rule in priority order (first match wins)
            for (rule, compiled_regex) in &compiled_rules {
                let pattern_lower = rule.description_pattern.to_lowercase();
                let matches = match rule.match_type.as_str() {
                    "exact" => desc_lower == pattern_lower,
                    "contains" => desc_lower.contains(&pattern_lower),
                    "starts_with" => desc_lower.starts_with(&pattern_lower),
                    "ends_with" => desc_lower.ends_with(&pattern_lower),
                    "regex" => compiled_regex
                        .as_ref()
                        .map(|r| r.is_match(&txn.description))
                        .unwrap_or(false),
                    _ => false,
                };

                if matches {
                    // Mark transaction as auto-matched
                    sqlx::query(
                        r#"
                        UPDATE bank_transactions
                        SET status = $2
                        WHERE transaction_id = $1
                        "#,
                    )
                    .bind(txn.transaction_id)
                    .bind(TransactionStatus::Matched.as_str())
                    .execute(self.pool())
                    .await
                    .map_err(|e| {
                        AppError::DatabaseError(anyhow::anyhow!(
                            "Failed to update transaction: {}",
                            e
                        ))
                    })?;

                    matched_count += 1;
                    info!(
                        transaction_id = %txn.transaction_id,
                        rule_name = %rule.name,
                        "Transaction auto-matched by rule"
                    );

                    // First match wins - stop checking rules for this transaction
                    break;
                }
            }
        }

        timer.observe_duration();
        info!(
            statement_id = %statement_id,
            matched_count = matched_count,
            "Applied matching rules"
        );

        Ok(matched_count)
    }
}
