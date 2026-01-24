//! Database service for reconciliation-service.

#![allow(clippy::too_many_arguments)]

use crate::grpc::proto;
use crate::models::{
    Adjustment, AdjustmentType, BankAccount, BankStatement, BankTransaction, MatchType,
    MatchingRule, Reconciliation, StatementStatus, TransactionMatch, TransactionStatus,
};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use service_core::error::AppError;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

/// Extracted transaction data from GenAI parsing.
#[derive(Debug, Clone)]
pub struct ExtractedTransaction {
    pub transaction_date: NaiveDate,
    pub description: String,
    pub reference: Option<String>,
    pub amount: Decimal,
    pub running_balance: Option<Decimal>,
    pub extraction_confidence: Option<f64>,
}

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool.
    #[instrument(skip(database_url), fields(service = "reconciliation-service"))]
    pub async fn new(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self, AppError> {
        info!(
            max_connections = max_connections,
            min_connections = min_connections,
            "Connecting to PostgreSQL"
        );

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to connect: {}", e)))?;

        info!("PostgreSQL connection pool established");

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check database health.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["health_check"])
            .start_timer();

        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Health check failed: {}", e)))?;

        timer.observe_duration();
        Ok(())
    }

    /// Run database migrations.
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<(), AppError> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Migration failed: {}", e)))?;
        info!("Database migrations completed");
        Ok(())
    }

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
        .fetch_one(&self.pool)
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
        .fetch_optional(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .fetch_one(&self.pool)
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
        .fetch_optional(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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

    /// Get the statement for a given bank transaction.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, transaction_id = %transaction_id))]
    pub async fn get_statement_by_transaction(
        &self,
        tenant_id: &str,
        transaction_id: &str,
    ) -> Result<Option<BankStatement>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_statement_by_transaction"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to get statement by transaction: {}",
                e
            ))
        })?;

        timer.observe_duration();
        Ok(statement)
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
        .fetch_optional(&self.pool)
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
        .execute(&self.pool)
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
        .execute(&self.pool)
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
        .fetch_one(&self.pool)
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
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create transaction: {}", e)))?;
            count += 1;
        }

        timer.observe_duration();
        info!(statement_id = %statement_id, count = %count, "Extracted transactions created");

        Ok(count)
    }

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_bank_transaction"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get bank transaction: {}", e))
        })?;

        timer.observe_duration();
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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_staged_transactions"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get staged transactions: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_staged_transaction"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update staged transaction: {}", e)))?;

        timer.observe_duration();

        Ok(transaction)
    }

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
        .fetch_one(&self.pool)
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
        .fetch_optional(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .execute(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
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
                    .execute(&self.pool)
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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["match_transaction"])
            .start_timer();

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
            .fetch_one(&self.pool)
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to update transaction status: {}",
                e
            ))
        })?;

        timer.observe_duration();

        Ok(matches)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_transaction_id = %bank_transaction_id))]
    pub async fn unmatch_transaction(
        &self,
        tenant_id: &str,
        bank_transaction_id: &str,
    ) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["unmatch_transaction"])
            .start_timer();

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
        .execute(&self.pool)
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to update transaction status: {}",
                e
            ))
        })?;

        timer.observe_duration();

        Ok(())
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_transaction_id = %bank_transaction_id))]
    pub async fn exclude_transaction(
        &self,
        tenant_id: &str,
        bank_transaction_id: &str,
        _reason: Option<&str>,
    ) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["exclude_transaction"])
            .start_timer();

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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to exclude transaction: {}", e))
        })?;

        timer.observe_duration();

        Ok(())
    }

    // =========================================================================
    // Reconciliation Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn start_reconciliation(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
        period_start: &str,
        period_end: &str,
        expected_balance_str: Option<&str>,
    ) -> Result<Reconciliation, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["start_reconciliation"])
            .start_timer();

        let reconciliation_id = Uuid::new_v4();
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;
        let start_date = NaiveDate::parse_from_str(period_start, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid period_start format")))?;
        let end_date = NaiveDate::parse_from_str(period_end, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid period_end format")))?;

        // Parse expected balance from ledger, default to zero if not provided
        let expected_balance = expected_balance_str
            .map(Decimal::from_str)
            .transpose()
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid expected_balance format")))?
            .unwrap_or(Decimal::ZERO);
        let actual_balance = Decimal::ZERO;

        let reconciliation = sqlx::query_as::<_, Reconciliation>(
            r#"
            INSERT INTO reconciliations (reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status, matched_count, unmatched_count, started_utc, completed_utc
            "#,
        )
        .bind(reconciliation_id)
        .bind(account_uuid)
        .bind(tenant_uuid)
        .bind(start_date)
        .bind(end_date)
        .bind(expected_balance)
        .bind(actual_balance)
        .bind(expected_balance - actual_balance)
        .bind("in_progress")
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to start reconciliation: {}", e)))?;

        timer.observe_duration();
        info!(reconciliation_id = %reconciliation.reconciliation_id, "Reconciliation started");

        Ok(reconciliation)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn get_reconciliation(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
    ) -> Result<Option<Reconciliation>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_reconciliation"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let recon_uuid = Uuid::from_str(reconciliation_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid reconciliation_id")))?;

        let reconciliation = sqlx::query_as::<_, Reconciliation>(
            r#"
            SELECT reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status, matched_count, unmatched_count, started_utc, completed_utc
            FROM reconciliations
            WHERE tenant_id = $1 AND reconciliation_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(recon_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get reconciliation: {}", e)))?;

        timer.observe_duration();

        Ok(reconciliation)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, bank_account_id = %bank_account_id))]
    pub async fn list_reconciliations(
        &self,
        tenant_id: &str,
        bank_account_id: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Reconciliation>, Option<String>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_reconciliations"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let account_uuid = Uuid::from_str(bank_account_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid bank_account_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let reconciliations = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, Reconciliation>(
                r#"
                SELECT reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status, matched_count, unmatched_count, started_utc, completed_utc
                FROM reconciliations
                WHERE tenant_id = $1 AND bank_account_id = $2 AND reconciliation_id > $3
                ORDER BY reconciliation_id
                LIMIT $4
                "#,
            )
            .bind(tenant_uuid)
            .bind(account_uuid)
            .bind(cursor_uuid)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, Reconciliation>(
                r#"
                SELECT reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status, matched_count, unmatched_count, started_utc, completed_utc
                FROM reconciliations
                WHERE tenant_id = $1 AND bank_account_id = $2
                ORDER BY reconciliation_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(account_uuid)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list reconciliations: {}", e)))?;

        timer.observe_duration();

        let has_more = reconciliations.len() > limit as usize;
        let mut reconciliations = reconciliations;
        if has_more {
            reconciliations.pop();
        }
        let next_token = if has_more {
            reconciliations
                .last()
                .map(|r| r.reconciliation_id.to_string())
        } else {
            None
        };

        Ok((reconciliations, next_token))
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn complete_reconciliation(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
    ) -> Result<Reconciliation, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["complete_reconciliation"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let recon_uuid = Uuid::from_str(reconciliation_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid reconciliation_id")))?;

        let reconciliation = sqlx::query_as::<_, Reconciliation>(
            r#"
            UPDATE reconciliations
            SET status = 'completed', completed_utc = NOW()
            WHERE tenant_id = $1 AND reconciliation_id = $2 AND status = 'in_progress'
            RETURNING reconciliation_id, bank_account_id, tenant_id, period_start, period_end, expected_balance, actual_balance, difference, status, matched_count, unmatched_count, started_utc, completed_utc
            "#,
        )
        .bind(tenant_uuid)
        .bind(recon_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to complete reconciliation: {}", e)))?
        .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Reconciliation not in progress")))?;

        // Update bank account's last_reconciled_date and last_reconciled_balance
        sqlx::query(
            r#"
            UPDATE bank_accounts
            SET last_reconciled_date = $3,
                last_reconciled_balance = $4,
                updated_utc = NOW()
            WHERE tenant_id = $1 AND bank_account_id = $2
            "#,
        )
        .bind(tenant_uuid)
        .bind(reconciliation.bank_account_id)
        .bind(reconciliation.period_end)
        .bind(reconciliation.actual_balance)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to update bank account: {}", e))
        })?;

        timer.observe_duration();
        info!(
            reconciliation_id = %reconciliation.reconciliation_id,
            bank_account_id = %reconciliation.bank_account_id,
            period_end = %reconciliation.period_end,
            "Reconciliation completed, bank account updated"
        );

        Ok(reconciliation)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn abandon_reconciliation(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
    ) -> Result<(), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["abandon_reconciliation"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let recon_uuid = Uuid::from_str(reconciliation_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid reconciliation_id")))?;

        sqlx::query(
            r#"
            UPDATE reconciliations
            SET status = 'abandoned'
            WHERE tenant_id = $1 AND reconciliation_id = $2 AND status = 'in_progress'
            "#,
        )
        .bind(tenant_uuid)
        .bind(recon_uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to abandon reconciliation: {}", e))
        })?;

        timer.observe_duration();

        Ok(())
    }

    // =========================================================================
    // Adjustment Operations
    // =========================================================================

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn create_adjustment(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
        adjustment_type: proto::AdjustmentType,
        description: &str,
        amount: &str,
    ) -> Result<Adjustment, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_adjustment"])
            .start_timer();

        let adjustment_id = Uuid::new_v4();
        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let recon_uuid = Uuid::from_str(reconciliation_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid reconciliation_id")))?;
        let amount_decimal = Decimal::from_str(amount)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid amount")))?;

        let adjustment = sqlx::query_as::<_, Adjustment>(
            r#"
            INSERT INTO adjustments (adjustment_id, reconciliation_id, tenant_id, adjustment_type, description, amount)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING adjustment_id, reconciliation_id, tenant_id, adjustment_type, description, amount, ledger_entry_id, created_utc
            "#,
        )
        .bind(adjustment_id)
        .bind(recon_uuid)
        .bind(tenant_uuid)
        .bind(AdjustmentType::from_proto(adjustment_type).as_str())
        .bind(description)
        .bind(amount_decimal)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create adjustment: {}", e)))?;

        timer.observe_duration();
        info!(adjustment_id = %adjustment.adjustment_id, "Adjustment created");

        Ok(adjustment)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn list_adjustments(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<(Vec<Adjustment>, Option<String>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_adjustments"])
            .start_timer();

        let tenant_uuid = Uuid::from_str(tenant_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid tenant_id")))?;
        let recon_uuid = Uuid::from_str(reconciliation_id)
            .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid reconciliation_id")))?;
        let limit = page_size.clamp(1, 100) as i64;

        let adjustments = if let Some(cursor) = page_token {
            let cursor_uuid = Uuid::from_str(cursor)
                .map_err(|_| AppError::BadRequest(anyhow::anyhow!("Invalid page_token")))?;
            sqlx::query_as::<_, Adjustment>(
                r#"
                SELECT adjustment_id, reconciliation_id, tenant_id, adjustment_type, description, amount, ledger_entry_id, created_utc
                FROM adjustments
                WHERE tenant_id = $1 AND reconciliation_id = $2 AND adjustment_id > $3
                ORDER BY adjustment_id
                LIMIT $4
                "#,
            )
            .bind(tenant_uuid)
            .bind(recon_uuid)
            .bind(cursor_uuid)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, Adjustment>(
                r#"
                SELECT adjustment_id, reconciliation_id, tenant_id, adjustment_type, description, amount, ledger_entry_id, created_utc
                FROM adjustments
                WHERE tenant_id = $1 AND reconciliation_id = $2
                ORDER BY adjustment_id
                LIMIT $3
                "#,
            )
            .bind(tenant_uuid)
            .bind(recon_uuid)
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list adjustments: {}", e)))?;

        timer.observe_duration();

        let has_more = adjustments.len() > limit as usize;
        let mut adjustments = adjustments;
        if has_more {
            adjustments.pop();
        }
        let next_token = if has_more {
            adjustments.last().map(|a| a.adjustment_id.to_string())
        } else {
            None
        };

        Ok((adjustments, next_token))
    }
}
