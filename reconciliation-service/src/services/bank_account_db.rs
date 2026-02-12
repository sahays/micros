//! Bank account and statement database operations for reconciliation-service.

#![allow(clippy::too_many_arguments)]

use crate::models::{BankAccount, BankStatement, StatementStatus, TransactionStatus};
use crate::services::database::{Database, ExtractedTransaction};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use service_core::error::AppError;
use std::str::FromStr;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
    // =========================================================================
    // Bank Account Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn create_bank_account(
        &self,
        tenant_id: &str,
        ledger_account_id: &str,
        bank_name: &str,
        account_number_masked: &str,
        currency: &str,
    ) -> Result<BankAccount, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_bank_account"])
            .start_timer();

        let bank_account_id = Uuid::new_v4();
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let ledger_uuid = Uuid::from_str(ledger_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid ledger_account_id")))?;

        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            INSERT INTO bank_accounts (bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
            "#,
        )
        .bind(bank_account_id)
        .bind(tenant_uuid)
        .bind(ledger_uuid)
        .bind(bank_name)
        .bind(account_number_masked)
        .bind(currency)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create bank account: {}", e)))?;

        timer.observe_duration();
        info!(bank_account_id = %account.bank_account_id, "Bank account created");

        Ok(account)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn get_bank_account(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
    ) -> Result<Option<BankAccount>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_bank_account"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;

        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            SELECT bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
            FROM bank_accounts
            WHERE tenant_id = $1 AND bank_account_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(account_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get bank account: {}", e)))?;

        timer.observe_duration();

        Ok(account)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn list_bank_accounts(
        &self,
        tenant_id: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<BankAccount>, Option<String>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_bank_accounts"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let accounts = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, BankAccount>(
                r#"
                SELECT bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
                FROM bank_accounts
                WHERE tenant_id = $1 AND bank_account_id > $2
                ORDER BY bank_account_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(cursor_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BankAccount>(
                r#"
                SELECT bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
                FROM bank_accounts
                WHERE tenant_id = $1
                ORDER BY bank_account_id
                LIMIT $2
                "#,
            )
            .bind(tenant_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list bank accounts: {}", e)))?;

        timer.observe_duration();

        let has_more = accounts.len() > limit as usize;
        let mut accounts = accounts;
        if has_more {
            accounts.pop();
        }
        let next_token = if has_more {
            accounts.last().map(|a| a.bank_account_id.to_string())
        } else {
            None
        };

        Ok((accounts, next_token))
    }

    /// Check if a bank account already exists for the given ledger account.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, ledger_account_id = %ledger_account_id))]
    pub async fn get_bank_account_by_ledger_id(
        &self,
        tenant_id: &str,
        ledger_account_id: &str,
    ) -> Result<Option<BankAccount>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_bank_account_by_ledger_id"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let ledger_uuid = Uuid::from_str(ledger_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid ledger_account_id")))?;

        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            SELECT bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
            FROM bank_accounts
            WHERE tenant_id = $1 AND ledger_account_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(ledger_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get bank account by ledger id: {}", e)))?;

        timer.observe_duration();

        Ok(account)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn update_bank_account(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
        bank_name: Option<&str>,
        account_number_masked: Option<&str>,
    ) -> Result<Option<BankAccount>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_bank_account"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;

        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            UPDATE bank_accounts
            SET bank_name = COALESCE($3, bank_name),
                account_number_masked = COALESCE($4, account_number_masked)
            WHERE tenant_id = $1 AND bank_account_id = $2
            RETURNING bank_account_id, tenant_id, ledger_account_id, bank_name, account_number_masked, currency, last_reconciled_date, last_reconciled_balance, created_utc, updated_utc
            "#,
        )
        .bind(tenant_uuid)
        .bind(account_uuid)
        .bind(bank_name)
        .bind(account_number_masked)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update bank account: {}", e)))?;

        timer.observe_duration();

        Ok(account)
    }

    // =========================================================================
    // Statement Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn create_statement(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
        document_id: &str,
    ) -> Result<BankStatement, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_statement"])
            .start_timer();

        let statement_id = Uuid::new_v4();
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;
        let doc_uuid = Uuid::from_str(document_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid document_id")))?;

        // Start with placeholder dates - will be updated after GenAI extraction
        let today = Utc::now().date_naive();

        let statement = sqlx::query_as::<_, BankStatement>(
            r#"
            INSERT INTO bank_statements (statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status)
            VALUES ($1, $2, $3, $4, $5, $6, 0, 0, $7)
            RETURNING statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
            "#,
        )
        .bind(statement_id)
        .bind(account_uuid)
        .bind(tenant_uuid)
        .bind(doc_uuid)
        .bind(today)
        .bind(today)
        .bind(StatementStatus::Uploaded.as_str())
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create statement: {}", e)))?;

        timer.observe_duration();
        info!(statement_id = %statement.statement_id, "Statement created");

        Ok(statement)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, statement_id = %statement_id))]
    pub async fn get_statement(
        &self,
        tenant_id: &str,
        statement_id: &str,
    ) -> Result<Option<BankStatement>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_statement"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        let statement = sqlx::query_as::<_, BankStatement>(
            r#"
            SELECT statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
            FROM bank_statements
            WHERE tenant_id = $1 AND statement_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(stmt_uuid)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get statement: {}", e)))?;

        timer.observe_duration();

        Ok(statement)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn list_statements(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<BankStatement>, Option<String>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_statements"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let statements = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, BankStatement>(
                r#"
                SELECT statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
                FROM bank_statements
                WHERE tenant_id = $1 AND bank_account_id = $2 AND statement_id > $3
                ORDER BY statement_id
                LIMIT $4
                "#,
            )
            .bind(tenant_uuid)
            .bind(account_uuid)
            .bind(cursor_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, BankStatement>(
                r#"
                SELECT statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
                FROM bank_statements
                WHERE tenant_id = $1 AND bank_account_id = $2
                ORDER BY statement_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(account_uuid)
            .bind(limit + 1)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list statements: {}", e)))?;

        timer.observe_duration();

        let has_more = statements.len() > limit as usize;
        let mut statements = statements;
        if has_more {
            statements.pop();
        }
        let next_token = if has_more {
            statements.last().map(|s| s.statement_id.to_string())
        } else {
            None
        };

        Ok((statements, next_token))
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, statement_id = %statement_id))]
    pub async fn commit_statement(
        &self,
        tenant_id: &str,
        statement_id: &str,
    ) -> Result<(BankStatement, i32), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["commit_statement"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        // Update statement status
        let statement = sqlx::query_as::<_, BankStatement>(
            r#"
            UPDATE bank_statements
            SET status = $3
            WHERE tenant_id = $1 AND statement_id = $2 AND status = 'staged'
            RETURNING statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
            "#,
        )
        .bind(tenant_uuid)
        .bind(stmt_uuid)
        .bind(StatementStatus::Committed.as_str())
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to commit statement: {}", e)))?
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Statement not in staged status")))?;

        // Update transaction statuses
        let result = sqlx::query(
            r#"
            UPDATE bank_transactions
            SET status = $2
            WHERE statement_id = $1 AND status = 'staged'
            "#,
        )
        .bind(stmt_uuid)
        .bind(TransactionStatus::Unmatched.as_str())
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to commit transactions: {}", e))
        })?;

        timer.observe_duration();

        Ok((statement, result.rows_affected() as i32))
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, statement_id = %statement_id))]
    pub async fn abandon_statement(
        &self,
        tenant_id: &str,
        statement_id: &str,
    ) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["abandon_statement"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        sqlx::query(
            r#"
            UPDATE bank_statements
            SET status = $3
            WHERE tenant_id = $1 AND statement_id = $2 AND status IN ('uploaded', 'extracting', 'staged')
            "#,
        )
        .bind(tenant_uuid)
        .bind(stmt_uuid)
        .bind(StatementStatus::Abandoned.as_str())
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to abandon statement: {}", e)))?;

        timer.observe_duration();

        Ok(())
    }

    /// Update statement with GenAI extraction results.
    #[instrument(skip(self), fields(statement_id = %statement_id))]
    pub async fn update_statement_extraction(
        &self,
        statement_id: &str,
        period_start: NaiveDate,
        period_end: NaiveDate,
        opening_balance: Decimal,
        closing_balance: Decimal,
        extraction_confidence: f64,
        status: StatementStatus,
        error_message: Option<&str>,
    ) -> Result<BankStatement, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_statement_extraction"])
            .start_timer();

        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        let statement = sqlx::query_as::<_, BankStatement>(
            r#"
            UPDATE bank_statements
            SET period_start = $2, period_end = $3, opening_balance = $4, closing_balance = $5, extraction_confidence = $6, status = $7, error_message = $8, updated_utc = NOW()
            WHERE statement_id = $1
            RETURNING statement_id, bank_account_id, tenant_id, document_id, period_start, period_end, opening_balance, closing_balance, status, error_message, extraction_confidence, created_utc, updated_utc
            "#,
        )
        .bind(stmt_uuid)
        .bind(period_start)
        .bind(period_end)
        .bind(opening_balance)
        .bind(closing_balance)
        .bind(extraction_confidence)
        .bind(status.as_str())
        .bind(error_message)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update statement extraction: {}", e)))?;

        timer.observe_duration();
        info!(statement_id = %statement.statement_id, status = %status.as_str(), "Statement extraction updated");

        Ok(statement)
    }

    /// Create extracted transactions from GenAI parsing results.
    #[instrument(skip(self, transactions), fields(statement_id = %statement_id, count = %transactions.len()))]
    pub async fn create_extracted_transactions(
        &self,
        tenant_id: &str,
        statement_id: &str,
        transactions: &[ExtractedTransaction],
    ) -> Result<i32, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_extracted_transactions"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let stmt_uuid = Uuid::from_str(statement_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid statement_id")))?;

        let mut count = 0;
        for txn in transactions {
            let txn_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO bank_transactions (transaction_id, statement_id, tenant_id, transaction_date, description, reference, amount, running_balance, status, extraction_confidence, is_modified)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(txn_id)
            .bind(stmt_uuid)
            .bind(tenant_uuid)
            .bind(txn.transaction_date)
            .bind(&txn.description)
            .bind(&txn.reference)
            .bind(txn.amount)
            .bind(txn.running_balance)
            .bind(TransactionStatus::Staged.as_str())
            .bind(txn.extraction_confidence)
            .bind(false)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create transaction: {}", e)))?;
            count += 1;
        }

        timer.observe_duration();
        info!(statement_id = %statement_id, count = %count, "Extracted transactions created");

        Ok(count)
    }
}
