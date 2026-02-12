//! Bank transaction database operations for reconciliation-service.

#![allow(clippy::too_many_arguments)]

use crate::models::{BankStatement, BankTransaction, TransactionMatch, TransactionStatus};
use crate::services::database::Database;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Transaction Operations
    // =========================================================================

    /// Get a single bank transaction by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, transaction_id = %transaction_id))]
    pub async fn get_bank_transaction(
        &self,
        tenant_id: &str,
        transaction_id: &str,
    ) -> Result<Option<BankTransaction>, AppError> {
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let txn_uuid = Uuid::from_str(transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid transaction_id")))?;

        let transaction = sqlx::query_as::<_, BankTransaction>(
            r#"
            SELECT transaction_id, statement_id, tenant_id, transaction_date, description,
                   reference, amount, running_balance, status, extraction_confidence,
                   is_modified, created_utc
            FROM bank_transactions
            WHERE tenant_id = $1 AND transaction_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(txn_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get bank transaction: {}", e))
        })?;
        Ok(transaction)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, statement_id = %statement_id))]
    pub async fn get_staged_transactions(
        &self,
        tenant_id: &str,
        statement_id: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<BankTransaction>, Option<String>), AppError> {
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let transactions = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, BankTransaction>(
                r#"
                SELECT transaction_id, statement_id, tenant_id, transaction_date, description, reference, amount, running_balance, status, extraction_confidence, is_modified, created_utc
                FROM bank_transactions
                WHERE tenant_id = $1 AND statement_id = $2 AND transaction_id > $3
                ORDER BY transaction_id
                LIMIT $4
                "#,
            )
            .bind(tenant_uuid)
            .bind(stmt_uuid)
            .bind(cursor_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BankTransaction>(
                r#"
                SELECT transaction_id, statement_id, tenant_id, transaction_date, description, reference, amount, running_balance, status, extraction_confidence, is_modified, created_utc
                FROM bank_transactions
                WHERE tenant_id = $1 AND statement_id = $2
                ORDER BY transaction_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(stmt_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get staged transactions: {}", e)))?;

        let has_more = transactions.len() > limit as usize;
        let mut transactions = transactions;
        if has_more {
            transactions.pop();
        }
        let next_token = if has_more {
            transactions.last().map(|t| t.transaction_id.to_string())
        } else {
            None
        };

        Ok((transactions, next_token))
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, transaction_id = %transaction_id))]
    pub async fn update_staged_transaction(
        &self,
        tenant_id: &str,
        transaction_id: &str,
        transaction_date: Option<&str>,
        description: Option<&str>,
        reference: Option<&str>,
        amount: Option<&str>,
    ) -> Result<Option<BankTransaction>, AppError> {
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let txn_uuid = Uuid::from_str(transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid transaction_id")))?;

        let parsed_date: Option<NaiveDate> = transaction_date
            .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
            .transpose()
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid date format")))?;

        let parsed_amount: Option<Decimal> = amount
            .map(Decimal::from_str)
            .transpose()
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid amount")))?;

        let transaction = sqlx::query_as::<_, BankTransaction>(
            r#"
            UPDATE bank_transactions
            SET transaction_date = COALESCE($3, transaction_date),
                description = COALESCE($4, description),
                reference = COALESCE($5, reference),
                amount = COALESCE($6, amount),
                is_modified = TRUE
            WHERE tenant_id = $1 AND transaction_id = $2 AND status = 'staged'
            RETURNING transaction_id, statement_id, tenant_id, transaction_date, description, reference, amount, running_balance, status, extraction_confidence, is_modified, created_utc
            "#,
        )
        .bind(tenant_uuid)
        .bind(txn_uuid)
        .bind(parsed_date)
        .bind(description)
        .bind(reference)
        .bind(parsed_amount)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update staged transaction: {}", e)))?;

        Ok(transaction)
    }

    /// Get the statement for a given bank transaction.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, transaction_id = %transaction_id))]
    pub async fn get_statement_by_transaction(
        &self,
        tenant_id: &str,
        transaction_id: &str,
    ) -> Result<Option<BankStatement>, AppError> {
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let txn_uuid = Uuid::from_str(transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid transaction_id")))?;

        let statement = sqlx::query_as::<_, BankStatement>(
            r#"
            SELECT s.statement_id, s.bank_account_id, s.tenant_id, s.document_id,
                   s.period_start, s.period_end, s.opening_balance, s.closing_balance,
                   s.status, s.error_message, s.extraction_confidence, s.created_utc, s.updated_utc
            FROM bank_statements s
            INNER JOIN bank_transactions t ON t.statement_id = s.statement_id
            WHERE t.tenant_id = $1 AND t.transaction_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(txn_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to get statement by transaction: {}",
                e
            ))
        })?;
        Ok(statement)
    }

    // =========================================================================
    // Transaction Matching Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_transaction_id = %bank_transaction_id))]
    pub async fn match_transaction(
        &self,
        tenant_id: &str,
        bank_transaction_id: &str,
        ledger_entry_ids: &[String],
        match_method: &str,
        matched_by: &str,
    ) -> Result<Vec<TransactionMatch>, AppError> {
        let _tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let txn_uuid = Uuid::from_str(bank_transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_transaction_id")))?;

        let mut matches = Vec::new();

        for ledger_id in ledger_entry_ids {
            let ledger_uuid = Uuid::from_str(ledger_id)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid ledger_entry_id")))?;
            let match_id = Uuid::new_v4();

            let m = sqlx::query_as::<_, TransactionMatch>(
                r#"
                INSERT INTO transaction_matches (match_id, bank_transaction_id, ledger_entry_id, match_method, matched_by)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING match_id, bank_transaction_id, ledger_entry_id, match_method, confidence_score, matched_by, matched_utc
                "#,
            )
            .bind(match_id)
            .bind(txn_uuid)
            .bind(ledger_uuid)
            .bind(match_method)
            .bind(matched_by)
            .fetch_one(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create match: {}", e)))?;

            matches.push(m);
        }

        // Update transaction status
        let status = if match_method == "manual" {
            TransactionStatus::ManuallyMatched.as_str()
        } else {
            TransactionStatus::Matched.as_str()
        };

        sqlx::query(
            r#"
            UPDATE bank_transactions
            SET status = $2
            WHERE transaction_id = $1
            "#,
        )
        .bind(txn_uuid)
        .bind(status)
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to update transaction status: {}",
                e
            ))
        })?;

        Ok(matches)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_transaction_id = %bank_transaction_id))]
    pub async fn unmatch_transaction(
        &self,
        tenant_id: &str,
        bank_transaction_id: &str,
    ) -> Result<(), AppError> {
        let txn_uuid = Uuid::from_str(bank_transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_transaction_id")))?;

        // Delete matches
        sqlx::query(
            r#"
            DELETE FROM transaction_matches
            WHERE bank_transaction_id = $1
            "#,
        )
        .bind(txn_uuid)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to delete matches: {}", e)))?;

        // Update transaction status
        sqlx::query(
            r#"
            UPDATE bank_transactions
            SET status = $2
            WHERE transaction_id = $1
            "#,
        )
        .bind(txn_uuid)
        .bind(TransactionStatus::Unmatched.as_str())
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to update transaction status: {}",
                e
            ))
        })?;

        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_transaction_id = %bank_transaction_id))]
    pub async fn exclude_transaction(
        &self,
        tenant_id: &str,
        bank_transaction_id: &str,
        _reason: Option<&str>,
    ) -> Result<(), AppError> {
        let txn_uuid = Uuid::from_str(bank_transaction_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_transaction_id")))?;

        sqlx::query(
            r#"
            UPDATE bank_transactions
            SET status = $2
            WHERE transaction_id = $1
            "#,
        )
        .bind(txn_uuid)
        .bind(TransactionStatus::Excluded.as_str())
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to exclude transaction: {}", e))
        })?;

        Ok(())
    }
}
