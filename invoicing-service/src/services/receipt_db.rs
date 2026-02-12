//! Receipt and statement database operations for invoicing-service.

use crate::models::{CreateReceipt, Invoice, ListReceiptsFilter, Receipt};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

use super::database::Database;

impl Database {
    // -------------------------------------------------------------------------
    // Receipt Operations
    // -------------------------------------------------------------------------

    /// Record a payment and create a receipt.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id, invoice_id = %input.invoice_id))]
    pub async fn record_payment(&self, input: &CreateReceipt) -> Result<Receipt, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["record_payment"])
            .start_timer();

        // Verify invoice is in issued status
        let invoice = self.get_invoice(input.tenant_id, input.invoice_id).await?;
        let invoice = match invoice {
            Some(inv) if inv.status == "issued" => inv,
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Can only record payments against issued invoices"
                )))
            }
            None => return Err(AppError::NotFound(anyhow::anyhow!("Invoice not found"))),
        };

        // Validate payment amount
        if input.amount > invoice.amount_due {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Payment amount {} exceeds amount due {}",
                input.amount,
                invoice.amount_due
            )));
        }

        let receipt_id = Uuid::new_v4();
        let receipt = sqlx::query_as::<_, Receipt>(
            r#"
            INSERT INTO receipts (
                receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                payment_method, payment_reference, payment_date, journal_id, notes
            )
            VALUES ($1, $2, next_receipt_number($2), $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                payment_method, payment_reference, payment_date, journal_id, notes, created_utc
            "#,
        )
        .bind(receipt_id)
        .bind(input.tenant_id)
        .bind(input.invoice_id)
        .bind(invoice.customer_id)
        .bind(input.amount)
        .bind(&invoice.currency)
        .bind(&input.payment_method)
        .bind(&input.payment_reference)
        .bind(input.payment_date)
        .bind(input.journal_id)
        .bind(&input.notes)
        .fetch_one(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to record payment: {}", e))
        })?;

        timer.observe_duration();

        info!(
            receipt_id = %receipt.receipt_id,
            receipt_number = %receipt.receipt_number,
            amount = %receipt.amount,
            "Payment recorded"
        );

        Ok(receipt)
    }

    /// Get a receipt by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, receipt_id = %receipt_id))]
    pub async fn get_receipt(
        &self,
        tenant_id: Uuid,
        receipt_id: Uuid,
    ) -> Result<Option<Receipt>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_receipt"])
            .start_timer();

        let receipt = sqlx::query_as::<_, Receipt>(
            r#"
            SELECT receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                payment_method, payment_reference, payment_date, journal_id, notes, created_utc
            FROM receipts
            WHERE tenant_id = $1 AND receipt_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(receipt_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get receipt: {}", e)))?;

        timer.observe_duration();

        Ok(receipt)
    }

    /// List receipts for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_receipts(
        &self,
        tenant_id: Uuid,
        filter: &ListReceiptsFilter,
    ) -> Result<Vec<Receipt>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_receipts"])
            .start_timer();

        let limit = filter.page_size.clamp(1, 100) as i64;

        let receipts = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, Receipt>(
                r#"
                SELECT receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                    payment_method, payment_reference, payment_date, journal_id, notes, created_utc
                FROM receipts
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR invoice_id = $2)
                  AND ($3::uuid IS NULL OR customer_id = $3)
                  AND ($4::date IS NULL OR payment_date >= $4)
                  AND ($5::date IS NULL OR payment_date <= $5)
                  AND receipt_id > $6
                ORDER BY receipt_id
                LIMIT $7
                "#,
            )
            .bind(tenant_id)
            .bind(filter.invoice_id)
            .bind(filter.customer_id)
            .bind(filter.start_date)
            .bind(filter.end_date)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, Receipt>(
                r#"
                SELECT receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                    payment_method, payment_reference, payment_date, journal_id, notes, created_utc
                FROM receipts
                WHERE tenant_id = $1
                  AND ($2::uuid IS NULL OR invoice_id = $2)
                  AND ($3::uuid IS NULL OR customer_id = $3)
                  AND ($4::date IS NULL OR payment_date >= $4)
                  AND ($5::date IS NULL OR payment_date <= $5)
                ORDER BY receipt_id
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(filter.invoice_id)
            .bind(filter.customer_id)
            .bind(filter.start_date)
            .bind(filter.end_date)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list receipts: {}", e)))?;

        timer.observe_duration();

        Ok(receipts)
    }

    // -------------------------------------------------------------------------
    // Statement Operations
    // -------------------------------------------------------------------------

    /// Calculate opening balance for a customer before a given date.
    /// Opening balance = sum of issued invoice totals - sum of payment amounts before period_start.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, customer_id = %customer_id))]
    pub async fn calculate_opening_balance(
        &self,
        tenant_id: Uuid,
        customer_id: Uuid,
        before_date: NaiveDate,
    ) -> Result<Decimal, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["calculate_opening_balance"])
            .start_timer();

        // Sum of issued invoice totals before period start
        let invoice_total: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(total), 0)
            FROM invoices
            WHERE tenant_id = $1
              AND customer_id = $2
              AND status IN ('issued', 'paid', 'overdue')
              AND issue_date < $3
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(before_date)
        .fetch_one(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to calculate invoice total: {}", e))
        })?;

        // Sum of payments before period start
        let payment_total: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM receipts
            WHERE tenant_id = $1
              AND customer_id = $2
              AND payment_date < $3
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(before_date)
        .fetch_one(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to calculate payment total: {}", e))
        })?;

        timer.observe_duration();

        let opening =
            invoice_total.unwrap_or(Decimal::ZERO) - payment_total.unwrap_or(Decimal::ZERO);
        Ok(opening)
    }

    /// Get invoices for a customer within a date range (for statement).
    #[instrument(skip(self), fields(tenant_id = %tenant_id, customer_id = %customer_id))]
    pub async fn get_invoices_for_statement(
        &self,
        tenant_id: Uuid,
        customer_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_invoices_for_statement"])
            .start_timer();

        let invoices = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            FROM invoices
            WHERE tenant_id = $1
              AND customer_id = $2
              AND status IN ('issued', 'paid', 'overdue')
              AND issue_date >= $3
              AND issue_date <= $4
            ORDER BY issue_date, invoice_number
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get invoices for statement: {}", e))
        })?;

        timer.observe_duration();

        Ok(invoices)
    }

    /// Get receipts for a customer within a date range (for statement).
    #[instrument(skip(self), fields(tenant_id = %tenant_id, customer_id = %customer_id))]
    pub async fn get_receipts_for_statement(
        &self,
        tenant_id: Uuid,
        customer_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Receipt>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_receipts_for_statement"])
            .start_timer();

        let receipts = sqlx::query_as::<_, Receipt>(
            r#"
            SELECT receipt_id, tenant_id, receipt_number, invoice_id, customer_id, amount, currency,
                payment_method, payment_reference, payment_date, journal_id, notes, created_utc
            FROM receipts
            WHERE tenant_id = $1
              AND customer_id = $2
              AND payment_date >= $3
              AND payment_date <= $4
            ORDER BY payment_date, receipt_number
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!(
                "Failed to get receipts for statement: {}",
                e
            ))
        })?;

        timer.observe_duration();

        Ok(receipts)
    }

    /// Get customer name and address for statement header.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, customer_id = %customer_id))]
    pub async fn get_customer_info(
        &self,
        tenant_id: Uuid,
        customer_id: Uuid,
    ) -> Result<Option<Invoice>, AppError> {
        // Get the most recent invoice for this customer to extract name/address
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            FROM invoices
            WHERE tenant_id = $1 AND customer_id = $2
            ORDER BY created_utc DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get customer info: {}", e))
        })?;

        Ok(invoice)
    }
}
