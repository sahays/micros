//! Reconciliation and adjustment database operations for reconciliation-service.

#![allow(clippy::too_many_arguments)]

use crate::grpc::proto;
use crate::models::{Adjustment, AdjustmentType, Reconciliation};
use crate::services::database::Database;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use std::str::FromStr;
use tracing::{info, instrument};
use uuid::Uuid;

impl Database {
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
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to start reconciliation: {}", e)))?;
        info!(reconciliation_id = %reconciliation.reconciliation_id, "Reconciliation started");

        Ok(reconciliation)
    }

    #[instrument(skip(self), fields(tenant_id = %tenant_id, reconciliation_id = %reconciliation_id))]
    pub async fn get_reconciliation(
        &self,
        tenant_id: &str,
        reconciliation_id: &str,
    ) -> Result<Option<Reconciliation>, AppError> {
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
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get reconciliation: {}", e)))?;

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
            .fetch_all(self.pool())
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
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list reconciliations: {}", e)))?;

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
        .fetch_optional(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to update bank account: {}", e))
        })?;
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
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to abandon reconciliation: {}", e))
        })?;

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
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create adjustment: {}", e)))?;
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
            .fetch_all(self.pool())
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
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list adjustments: {}", e)))?;

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
