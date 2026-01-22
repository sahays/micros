//! Database service for ledger-service.

use crate::models::{Account, AccountType, CreateAccount, Direction, LedgerEntry, PostEntry};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

/// Database connection pool wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool.
    #[instrument(skip(database_url), fields(service = "ledger-service"))]
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
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Health check failed: {}", e)))?;
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

    // -------------------------------------------------------------------------
    // Account Operations
    // -------------------------------------------------------------------------

    /// Create a new account.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id, account_code = %input.account_code))]
    pub async fn create_account(&self, input: &CreateAccount) -> Result<Account, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_account"])
            .start_timer();

        let account_id = Uuid::new_v4();
        let account = sqlx::query_as::<_, Account>(
            r#"
            INSERT INTO accounts (account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata, created_utc, closed_utc
            "#,
        )
        .bind(account_id)
        .bind(input.tenant_id)
        .bind(input.account_type.as_str())
        .bind(&input.account_code)
        .bind(&input.currency)
        .bind(input.allow_negative)
        .bind(&input.metadata)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                AppError::Conflict(anyhow::anyhow!(
                    "Account with code '{}' already exists for tenant",
                    input.account_code
                ))
            }
            _ => AppError::DatabaseError(anyhow::anyhow!("Failed to create account: {}", e)),
        })?;

        timer.observe_duration();

        info!(
            account_id = %account.account_id,
            account_type = %account.account_type,
            "Account created"
        );

        Ok(account)
    }

    /// Get an account by ID for a specific tenant.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, account_id = %account_id))]
    pub async fn get_account(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<Account>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_account"])
            .start_timer();

        let account = sqlx::query_as::<_, Account>(
            r#"
            SELECT account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata, created_utc, closed_utc
            FROM accounts
            WHERE tenant_id = $1 AND account_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get account: {}", e)))?;

        timer.observe_duration();

        Ok(account)
    }

    /// List accounts for a tenant with optional filters.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn list_accounts(
        &self,
        tenant_id: Uuid,
        account_type: Option<AccountType>,
        currency: Option<&str>,
        page_size: i32,
        page_token: Option<Uuid>,
    ) -> Result<Vec<Account>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_accounts"])
            .start_timer();

        let limit = page_size.min(100).max(1) as i64;

        // Build dynamic query based on filters
        let accounts = if let Some(cursor) = page_token {
            sqlx::query_as::<_, Account>(
                r#"
                SELECT account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata, created_utc, closed_utc
                FROM accounts
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR account_type = $2)
                  AND ($3::varchar IS NULL OR currency = $3)
                  AND account_id > $4
                ORDER BY account_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(account_type.map(|t| t.as_str()))
            .bind(currency)
            .bind(cursor)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, Account>(
                r#"
                SELECT account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata, created_utc, closed_utc
                FROM accounts
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR account_type = $2)
                  AND ($3::varchar IS NULL OR currency = $3)
                ORDER BY account_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(account_type.map(|t| t.as_str()))
            .bind(currency)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list accounts: {}", e)))?;

        timer.observe_duration();

        Ok(accounts)
    }

    // -------------------------------------------------------------------------
    // Transaction Operations
    // -------------------------------------------------------------------------

    /// Post a new transaction (multiple entries in a single journal).
    /// Validates that debits equal credits before inserting.
    #[instrument(skip(self, entries, metadata), fields(tenant_id = %tenant_id, entry_count = entries.len()))]
    pub async fn post_transaction(
        &self,
        tenant_id: Uuid,
        entries: &[PostEntry],
        effective_date: NaiveDate,
        idempotency_key: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<(Uuid, Vec<LedgerEntry>), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["post_transaction"])
            .start_timer();

        // Validate at least 2 entries
        if entries.len() < 2 {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Transaction must have at least 2 entries"
            )));
        }

        // Validate double-entry: sum of debits must equal sum of credits
        let mut debit_sum = Decimal::ZERO;
        let mut credit_sum = Decimal::ZERO;

        for entry in entries {
            if entry.amount <= Decimal::ZERO {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Entry amount must be positive"
                )));
            }
            match entry.direction {
                Direction::Debit => debit_sum += entry.amount,
                Direction::Credit => credit_sum += entry.amount,
            }
        }

        if debit_sum != credit_sum {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Double-entry violation: debits ({}) != credits ({})",
                debit_sum,
                credit_sum
            )));
        }

        // Check idempotency
        if let Some(key) = idempotency_key {
            let existing = sqlx::query_scalar::<_, Uuid>(
                "SELECT journal_id FROM ledger_entries WHERE idempotency_key = $1 LIMIT 1",
            )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::DatabaseError(anyhow::anyhow!("Failed to check idempotency: {}", e))
            })?;

            if let Some(journal_id) = existing {
                // Return existing transaction
                let entries = self.get_entries_by_journal(tenant_id, journal_id).await?;
                timer.observe_duration();
                return Ok((journal_id, entries));
            }
        }

        let journal_id = Uuid::new_v4();

        // Insert all entries in a single transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to begin transaction: {}", e))
        })?;

        let mut inserted_entries = Vec::with_capacity(entries.len());

        for (i, entry) in entries.iter().enumerate() {
            let entry_id = Uuid::new_v4();
            // Only first entry gets the idempotency key
            let key = if i == 0 { idempotency_key } else { None };

            let inserted = sqlx::query_as::<_, LedgerEntry>(
                r#"
                INSERT INTO ledger_entries (entry_id, tenant_id, journal_id, account_id, amount, direction, effective_date, idempotency_key, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING entry_id, tenant_id, journal_id, account_id, amount, direction, effective_date, posted_utc, idempotency_key, metadata
                "#,
            )
            .bind(entry_id)
            .bind(tenant_id)
            .bind(journal_id)
            .bind(entry.account_id)
            .bind(entry.amount)
            .bind(entry.direction.as_str())
            .bind(effective_date)
            .bind(key)
            .bind(&metadata)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| {
                AppError::DatabaseError(anyhow::anyhow!("Failed to insert entry: {}", e))
            })?;

            inserted_entries.push(inserted);
        }

        tx.commit().await.map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to commit transaction: {}", e))
        })?;

        timer.observe_duration();

        info!(
            journal_id = %journal_id,
            entry_count = entries.len(),
            total_amount = %debit_sum,
            "Transaction posted"
        );

        Ok((journal_id, inserted_entries))
    }

    /// Get all entries for a journal.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, journal_id = %journal_id))]
    pub async fn get_entries_by_journal(
        &self,
        tenant_id: Uuid,
        journal_id: Uuid,
    ) -> Result<Vec<LedgerEntry>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_entries_by_journal"])
            .start_timer();

        let entries = sqlx::query_as::<_, LedgerEntry>(
            r#"
            SELECT entry_id, tenant_id, journal_id, account_id, amount, direction, effective_date, posted_utc, idempotency_key, metadata
            FROM ledger_entries
            WHERE tenant_id = $1 AND journal_id = $2
            ORDER BY entry_id
            "#,
        )
        .bind(tenant_id)
        .bind(journal_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get entries: {}", e)))?;

        timer.observe_duration();

        Ok(entries)
    }

    /// List transactions (grouped by journal_id) with optional filters.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn list_transactions(
        &self,
        tenant_id: Uuid,
        account_id: Option<Uuid>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page_size: i32,
        page_token: Option<Uuid>,
    ) -> Result<Vec<(Uuid, Vec<LedgerEntry>)>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_transactions"])
            .start_timer();

        let limit = page_size.min(100).max(1) as i64;

        // First get distinct journal_ids
        let journal_ids: Vec<Uuid> = if let Some(cursor) = page_token {
            sqlx::query_scalar(
                r#"
                SELECT DISTINCT journal_id
                FROM ledger_entries
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR account_id = $2)
                  AND ($3::date IS NULL OR effective_date >= $3)
                  AND ($4::date IS NULL OR effective_date <= $4)
                  AND journal_id > $5
                ORDER BY journal_id
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(account_id)
            .bind(start_date)
            .bind(end_date)
            .bind(cursor)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_scalar(
                r#"
                SELECT DISTINCT journal_id
                FROM ledger_entries
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR account_id = $2)
                  AND ($3::date IS NULL OR effective_date >= $3)
                  AND ($4::date IS NULL OR effective_date <= $4)
                ORDER BY journal_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(account_id)
            .bind(start_date)
            .bind(end_date)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list journals: {}", e)))?;

        // Now fetch all entries for these journals
        let mut result = Vec::with_capacity(journal_ids.len());
        for journal_id in journal_ids {
            let entries = self.get_entries_by_journal(tenant_id, journal_id).await?;
            if !entries.is_empty() {
                result.push((journal_id, entries));
            }
        }

        timer.observe_duration();

        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Balance Operations
    // -------------------------------------------------------------------------

    /// Get balance for an account as of a specific date.
    /// Balance = SUM(debits) - SUM(credits)
    /// For Asset/Expense accounts: positive balance means debit balance
    /// For Liability/Equity/Revenue accounts: positive balance means credit balance (stored as negative)
    #[instrument(skip(self), fields(tenant_id = %tenant_id, account_id = %account_id))]
    pub async fn get_balance(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
        as_of_date: Option<NaiveDate>,
    ) -> Result<Option<(Decimal, String)>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_balance"])
            .start_timer();

        // First get the account to verify it exists and get currency
        let account = self.get_account(tenant_id, account_id).await?;
        let account = match account {
            Some(a) => a,
            None => return Ok(None),
        };

        // Calculate balance from entries
        let as_of = as_of_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let balance: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                SUM(CASE WHEN direction = 'debit' THEN amount ELSE -amount END),
                0
            )
            FROM ledger_entries
            WHERE tenant_id = $1
              AND account_id = $2
              AND effective_date <= $3
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(as_of)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get balance: {}", e)))?;

        timer.observe_duration();

        Ok(Some((balance.unwrap_or(Decimal::ZERO), account.currency)))
    }

    /// Get balances for multiple accounts.
    #[instrument(skip(self, account_ids), fields(tenant_id = %tenant_id, account_count = account_ids.len()))]
    pub async fn get_balances(
        &self,
        tenant_id: Uuid,
        account_ids: &[Uuid],
        as_of_date: Option<NaiveDate>,
    ) -> Result<Vec<(Uuid, Decimal, String)>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_balances"])
            .start_timer();

        let as_of = as_of_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Get all accounts first to verify they exist and get currencies
        let mut results = Vec::with_capacity(account_ids.len());

        for account_id in account_ids {
            if let Some((balance, currency)) =
                self.get_balance(tenant_id, *account_id, Some(as_of)).await?
            {
                results.push((*account_id, balance, currency));
            }
        }

        timer.observe_duration();

        Ok(results)
    }

    // -------------------------------------------------------------------------
    // Statement Operations
    // -------------------------------------------------------------------------

    /// Get account statement with running balance for a date range.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, account_id = %account_id))]
    pub async fn get_statement(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Option<(String, Decimal, Decimal, Vec<LedgerEntry>)>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_statement"])
            .start_timer();

        // Get account
        let account = self.get_account(tenant_id, account_id).await?;
        let account = match account {
            Some(a) => a,
            None => return Ok(None),
        };

        // Calculate opening balance (balance as of day before start_date)
        let opening_date = start_date.pred_opt().unwrap_or(start_date);
        let opening_balance: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                SUM(CASE WHEN direction = 'debit' THEN amount ELSE -amount END),
                0
            )
            FROM ledger_entries
            WHERE tenant_id = $1
              AND account_id = $2
              AND effective_date <= $3
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(opening_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get opening balance: {}", e))
        })?;

        let opening_balance = opening_balance.unwrap_or(Decimal::ZERO);

        // Get entries in date range
        let entries = sqlx::query_as::<_, LedgerEntry>(
            r#"
            SELECT entry_id, tenant_id, journal_id, account_id, amount, direction, effective_date, posted_utc, idempotency_key, metadata
            FROM ledger_entries
            WHERE tenant_id = $1
              AND account_id = $2
              AND effective_date >= $3
              AND effective_date <= $4
            ORDER BY effective_date, posted_utc
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get statement entries: {}", e))
        })?;

        // Calculate closing balance
        let mut closing_balance = opening_balance;
        for entry in &entries {
            match entry.direction.as_str() {
                "debit" => closing_balance += entry.amount,
                "credit" => closing_balance -= entry.amount,
                _ => {}
            }
        }

        timer.observe_duration();

        Ok(Some((
            account.currency,
            opening_balance,
            closing_balance,
            entries,
        )))
    }
}
