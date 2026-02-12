//! Account database operations.

use crate::models::{Account, AccountType, CreateAccount};
use crate::services::database::Database;
use crate::services::metrics::DB_QUERY_DURATION;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
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
        .fetch_one(self.pool())
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
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get account: {}", e)))?;

        timer.observe_duration();

        Ok(account)
    }

    /// P1: Get an account with its current balance.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, account_id = %account_id))]
    pub async fn get_account_with_balance(
        &self,
        tenant_id: Uuid,
        account_id: Uuid,
    ) -> Result<Option<(Account, Decimal)>, AppError> {
        let account = self.get_account(tenant_id, account_id).await?;
        let account = match account {
            Some(a) => a,
            None => return Ok(None),
        };

        // Get balance
        let balance_result = self.get_balance(tenant_id, account_id, None).await?;
        let balance = balance_result.map(|(b, _)| b).unwrap_or(Decimal::ZERO);

        Ok(Some((account, balance)))
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

        let limit = page_size.clamp(1, 100) as i64;

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
            .fetch_all(self.pool())
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
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list accounts: {}", e)))?;

        timer.observe_duration();

        Ok(accounts)
    }
}
