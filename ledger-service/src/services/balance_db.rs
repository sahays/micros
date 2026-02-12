//! Balance and statement database operations.

use crate::models::{AccountType, LedgerEntry};
use crate::services::database::Database;
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::instrument;
use uuid::Uuid;

/// Statement data returned by get_statement query.
/// Contains: (currency, opening_balance, closing_balance, entries)
pub(crate) type StatementData = (String, Decimal, Decimal, Vec<LedgerEntry>);

impl Database {
    // -------------------------------------------------------------------------
    // Balance Operations
    // -------------------------------------------------------------------------

    /// Get balance for an account as of a specific date.
    /// P2: Balance calculation considers account type:
    /// - Asset/Expense (debit-normal): balance = debits - credits (positive = normal)
    /// - Liability/Equity/Revenue (credit-normal): balance = credits - debits (positive = normal)
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

        // First get the account to verify it exists and get currency/type
        let account = self.get_account(tenant_id, account_id).await?;
        let account = match account {
            Some(a) => a,
            None => return Ok(None),
        };

        // Calculate balance from entries
        let as_of = as_of_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        // Calculate raw balance (debits - credits)
        let raw_balance: Option<Decimal> = sqlx::query_scalar(
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
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get balance: {}", e)))?;

        let raw = raw_balance.unwrap_or(Decimal::ZERO);

        // P2: Adjust sign based on account type
        // For credit-normal accounts, negate to show positive balance
        let account_type = AccountType::from_string(&account.account_type);
        let is_debit_normal = matches!(account_type, AccountType::Asset | AccountType::Expense);
        let balance = if is_debit_normal { raw } else { -raw };

        timer.observe_duration();

        Ok(Some((balance, account.currency)))
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
            if let Some((balance, currency)) = self
                .get_balance(tenant_id, *account_id, Some(as_of))
                .await?
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
    ) -> Result<Option<StatementData>, AppError> {
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
        .fetch_one(self.pool())
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
        .fetch_all(self.pool())
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
