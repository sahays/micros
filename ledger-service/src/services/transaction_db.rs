//! Transaction and entry database operations.

use crate::models::{Account, AccountType, Direction, LedgerEntry, PostEntry};
use crate::services::database::Database;
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
    // -------------------------------------------------------------------------
    // Transaction Operations
    // -------------------------------------------------------------------------

    /// Post a new transaction (multiple entries in a single journal).
    /// Validates that debits equal credits, all accounts belong to tenant,
    /// all accounts have same currency, and no account would go negative
    /// (unless allow_negative is set).
    /// Returns (journal_id, entries, currency).
    #[instrument(skip(self, entries, metadata), fields(tenant_id = %tenant_id, entry_count = entries.len()))]
    pub async fn post_transaction(
        &self,
        tenant_id: Uuid,
        entries: &[PostEntry],
        effective_date: NaiveDate,
        idempotency_key: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<(Uuid, Vec<LedgerEntry>, String), AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["post_transaction"])
            .start_timer();

        // Validate at least 2 entries
        if entries.len() < 2 {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Transaction must have at least 2 entries"
            )));
        }

        // Collect unique account IDs for validation
        let account_ids: Vec<Uuid> = entries.iter().map(|e| e.account_id).collect();

        // P0: Validate all accounts exist and belong to tenant
        // P1: Also fetch accounts to check currency consistency and allow_negative
        let accounts: Vec<Account> = sqlx::query_as::<_, Account>(
            r#"
            SELECT account_id, tenant_id, account_type, account_code, currency, allow_negative, metadata, created_utc, closed_utc
            FROM accounts
            WHERE tenant_id = $1 AND account_id = ANY($2)
            "#,
        )
        .bind(tenant_id)
        .bind(&account_ids)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to fetch accounts: {}", e)))?;

        // Build a map for quick lookup
        let account_map: std::collections::HashMap<Uuid, &Account> =
            accounts.iter().map(|a| (a.account_id, a)).collect();

        // Validate all accounts exist and belong to tenant
        for entry in entries {
            if !account_map.contains_key(&entry.account_id) {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Account {} does not exist or does not belong to tenant",
                    entry.account_id
                )));
            }
        }

        // P1: Validate currency consistency - all accounts must have same currency
        let first_currency = &accounts[0].currency;
        for account in &accounts {
            if account.currency != *first_currency {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Currency mismatch: account {} has currency {} but expected {}",
                    account.account_id,
                    account.currency,
                    first_currency
                )));
            }
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

        // P1: Validate negative balance constraints
        // Calculate impact on each account and check if resulting balance would go negative
        for entry in entries {
            let account = account_map.get(&entry.account_id).unwrap();

            // Skip check if account allows negative balances
            if account.allow_negative {
                continue;
            }

            // Get current balance
            let current_balance: Option<Decimal> = sqlx::query_scalar(
                r#"
                SELECT COALESCE(
                    SUM(CASE WHEN direction = 'debit' THEN amount ELSE -amount END),
                    0
                )
                FROM ledger_entries
                WHERE tenant_id = $1 AND account_id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(entry.account_id)
            .fetch_one(self.pool())
            .await
            .map_err(|e| {
                AppError::DatabaseError(anyhow::anyhow!("Failed to get balance: {}", e))
            })?;

            let current = current_balance.unwrap_or(Decimal::ZERO);

            // Calculate impact based on account type and direction
            // Asset/Expense: debit increases, credit decreases (normal debit balance)
            // Liability/Equity/Revenue: credit increases, debit decreases (normal credit balance)
            let impact = match entry.direction {
                Direction::Debit => entry.amount,
                Direction::Credit => -entry.amount,
            };

            // For normal-debit accounts (asset/expense), balance should stay >= 0
            // For normal-credit accounts (liability/equity/revenue), balance should stay <= 0
            let new_balance = current + impact;

            let account_type = AccountType::from_string(&account.account_type);
            let is_debit_normal = matches!(account_type, AccountType::Asset | AccountType::Expense);

            if is_debit_normal && new_balance < Decimal::ZERO {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Insufficient balance in account {}: current {}, would become {}",
                    entry.account_id,
                    current,
                    new_balance
                )));
            } else if !is_debit_normal && new_balance > Decimal::ZERO {
                // Credit-normal accounts should not have debit balance
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Insufficient balance in account {}: current {}, would become {}",
                    entry.account_id,
                    current,
                    new_balance
                )));
            }
        }

        // Check idempotency - use transaction to handle race condition
        let mut tx = self.pool().begin().await.map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to begin transaction: {}", e))
        })?;

        if let Some(key) = idempotency_key {
            let existing = sqlx::query_scalar::<_, Uuid>(
                "SELECT journal_id FROM ledger_entries WHERE idempotency_key = $1 LIMIT 1",
            )
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                AppError::DatabaseError(anyhow::anyhow!("Failed to check idempotency: {}", e))
            })?;

            if let Some(journal_id) = existing {
                // Return existing transaction
                tx.rollback().await.ok();
                let entries = self.get_entries_by_journal(tenant_id, journal_id).await?;
                timer.observe_duration();
                return Ok((journal_id, entries, first_currency.clone()));
            }
        }

        let journal_id = Uuid::new_v4();
        let mut inserted_entries = Vec::with_capacity(entries.len());

        for (i, entry) in entries.iter().enumerate() {
            let entry_id = Uuid::new_v4();
            // Only first entry gets the idempotency key
            let key = if i == 0 { idempotency_key } else { None };

            let result = sqlx::query_as::<_, LedgerEntry>(
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
            .await;

            match result {
                Ok(inserted) => inserted_entries.push(inserted),
                Err(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                    // P2: Idempotency key race condition - another request won
                    // Roll back and return the existing transaction
                    tx.rollback().await.ok();
                    if let Some(key) = idempotency_key {
                        let existing_journal = sqlx::query_scalar::<_, Uuid>(
                            "SELECT journal_id FROM ledger_entries WHERE idempotency_key = $1 LIMIT 1",
                        )
                        .bind(key)
                        .fetch_optional(self.pool())
                        .await
                        .map_err(|e| {
                            AppError::DatabaseError(anyhow::anyhow!("Failed to fetch existing: {}", e))
                        })?;

                        if let Some(jid) = existing_journal {
                            let entries = self.get_entries_by_journal(tenant_id, jid).await?;
                            timer.observe_duration();
                            return Ok((jid, entries, first_currency.clone()));
                        }
                    }
                    return Err(AppError::Conflict(anyhow::anyhow!(
                        "Duplicate idempotency key"
                    )));
                }
                Err(e) => {
                    return Err(AppError::DatabaseError(anyhow::anyhow!(
                        "Failed to insert entry: {}",
                        e
                    )));
                }
            }
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

        Ok((journal_id, inserted_entries, first_currency.clone()))
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
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get entries: {}", e)))?;

        timer.observe_duration();

        Ok(entries)
    }

    /// List transactions (grouped by journal_id) with optional filters.
    /// P3: Orders by effective_date DESC, posted_utc DESC (most recent first).
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

        let limit = page_size.clamp(1, 100) as i64;

        // P3: Get distinct journal_ids ordered by effective_date DESC, posted_utc DESC
        // Use a subquery to get the first entry's timestamp for each journal
        let journal_ids: Vec<Uuid> = if let Some(cursor) = page_token {
            // For cursor pagination with descending order, we use a subquery to get
            // the cursor's effective_date/posted_utc and then filter for earlier entries
            sqlx::query_scalar(
                r#"
                WITH journal_order AS (
                    SELECT DISTINCT ON (journal_id)
                        journal_id,
                        effective_date,
                        posted_utc
                    FROM ledger_entries
                    WHERE tenant_id = $1
                      AND ($2::uuid IS NULL OR account_id = $2)
                      AND ($3::date IS NULL OR effective_date >= $3)
                      AND ($4::date IS NULL OR effective_date <= $4)
                    ORDER BY journal_id, posted_utc
                ),
                cursor_pos AS (
                    SELECT effective_date, posted_utc FROM journal_order WHERE journal_id = $5
                )
                SELECT jo.journal_id
                FROM journal_order jo, cursor_pos cp
                WHERE (jo.effective_date, jo.posted_utc, jo.journal_id) < (cp.effective_date, cp.posted_utc, $5)
                ORDER BY jo.effective_date DESC, jo.posted_utc DESC, jo.journal_id DESC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(account_id)
            .bind(start_date)
            .bind(end_date)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_scalar(
                r#"
                SELECT journal_id
                FROM (
                    SELECT DISTINCT ON (journal_id)
                        journal_id,
                        effective_date,
                        posted_utc
                    FROM ledger_entries
                    WHERE tenant_id = $1
                      AND ($2::uuid IS NULL OR account_id = $2)
                      AND ($3::date IS NULL OR effective_date >= $3)
                      AND ($4::date IS NULL OR effective_date <= $4)
                    ORDER BY journal_id, posted_utc
                ) sub
                ORDER BY effective_date DESC, posted_utc DESC, journal_id DESC
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(account_id)
            .bind(start_date)
            .bind(end_date)
            .bind(limit)
            .fetch_all(self.pool())
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
}
