//! Tax rate database operations for invoicing-service.

use crate::models::{CreateTaxRate, TaxRate, UpdateTaxRate};
use chrono::NaiveDate;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

use super::database::Database;

impl Database {
    /// Create a new tax rate.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_tax_rate(&self, input: &CreateTaxRate) -> Result<TaxRate, AppError> {
        let tax_rate_id = Uuid::new_v4();
        let tax_rate = sqlx::query_as::<_, TaxRate>(
            r#"
            INSERT INTO tax_rates (tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
            "#,
        )
        .bind(tax_rate_id)
        .bind(input.tenant_id)
        .bind(&input.name)
        .bind(input.rate)
        .bind(&input.calculation)
        .bind(input.effective_from)
        .bind(input.effective_to)
        .bind(true)
        .fetch_one(self.pool())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => {
                AppError::Conflict(anyhow::anyhow!(
                    "Tax rate '{}' already exists for this period",
                    input.name
                ))
            }
            _ => AppError::DatabaseError(anyhow::anyhow!("Failed to create tax rate: {}", e)),
        })?;

        info!(tax_rate_id = %tax_rate.tax_rate_id, name = %tax_rate.name, "Tax rate created");

        Ok(tax_rate)
    }

    /// Get a tax rate by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, tax_rate_id = %tax_rate_id))]
    pub async fn get_tax_rate(
        &self,
        tenant_id: Uuid,
        tax_rate_id: Uuid,
    ) -> Result<Option<TaxRate>, AppError> {
        let tax_rate = sqlx::query_as::<_, TaxRate>(
            r#"
            SELECT tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
            FROM tax_rates
            WHERE tenant_id = $1 AND tax_rate_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(tax_rate_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get tax rate: {}", e)))?;

        Ok(tax_rate)
    }

    /// List tax rates for a tenant.
    #[instrument(skip(self), fields(tenant_id = %tenant_id))]
    pub async fn list_tax_rates(
        &self,
        tenant_id: Uuid,
        active_only: bool,
        as_of_date: Option<NaiveDate>,
        page_size: i32,
        page_token: Option<Uuid>,
    ) -> Result<Vec<TaxRate>, AppError> {
        let limit = page_size.clamp(1, 100) as i64;
        let as_of = as_of_date.unwrap_or_else(|| chrono::Utc::now().date_naive());

        let tax_rates = if let Some(cursor) = page_token {
            sqlx::query_as::<_, TaxRate>(
                r#"
                SELECT tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
                FROM tax_rates
                WHERE tenant_id = $1
                  AND ($2::bool = FALSE OR active = TRUE)
                  AND effective_from <= $3
                  AND (effective_to IS NULL OR effective_to >= $3)
                  AND tax_rate_id > $4
                ORDER BY tax_rate_id
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(active_only)
            .bind(as_of)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, TaxRate>(
                r#"
                SELECT tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
                FROM tax_rates
                WHERE tenant_id = $1
                  AND ($2::bool = FALSE OR active = TRUE)
                  AND effective_from <= $3
                  AND (effective_to IS NULL OR effective_to >= $3)
                ORDER BY tax_rate_id
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(active_only)
            .bind(as_of)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list tax rates: {}", e)))?;

        Ok(tax_rates)
    }

    /// Update a tax rate.
    #[instrument(skip(self, input), fields(tenant_id = %tenant_id, tax_rate_id = %tax_rate_id))]
    pub async fn update_tax_rate(
        &self,
        tenant_id: Uuid,
        tax_rate_id: Uuid,
        input: &UpdateTaxRate,
    ) -> Result<Option<TaxRate>, AppError> {
        let tax_rate = sqlx::query_as::<_, TaxRate>(
            r#"
            UPDATE tax_rates
            SET name = COALESCE($3, name),
                rate = COALESCE($4, rate),
                calculation = COALESCE($5, calculation),
                effective_from = COALESCE($6, effective_from),
                effective_to = $7,
                active = COALESCE($8, active)
            WHERE tenant_id = $1 AND tax_rate_id = $2
            RETURNING tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
            "#,
        )
        .bind(tenant_id)
        .bind(tax_rate_id)
        .bind(&input.name)
        .bind(input.rate)
        .bind(&input.calculation)
        .bind(input.effective_from)
        .bind(input.effective_to)
        .bind(input.active)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update tax rate: {}", e)))?;

        Ok(tax_rate)
    }
}
