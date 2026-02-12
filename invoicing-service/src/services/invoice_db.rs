//! Invoice and line item database operations for invoicing-service.

use crate::models::{
    CreateInvoice, CreateLineItem, Invoice, LineItem, ListInvoicesFilter, UpdateInvoice,
    UpdateLineItem,
};
use crate::services::metrics::DB_QUERY_DURATION;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use service_core::error::AppError;
use tracing::{info, instrument};
use uuid::Uuid;

use super::database::Database;

impl Database {
    // -------------------------------------------------------------------------
    // Invoice Operations
    // -------------------------------------------------------------------------

    /// Create a new draft invoice.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_invoice(&self, input: &CreateInvoice) -> Result<Invoice, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_invoice"])
            .start_timer();

        let invoice_id = Uuid::new_v4();
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices (
                invoice_id, tenant_id, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, due_date, notes, reference_invoice_id, metadata
            )
            VALUES ($1, $2, $3, 'draft', $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            "#,
        )
        .bind(invoice_id)
        .bind(input.tenant_id)
        .bind(&input.invoice_type)
        .bind(input.customer_id)
        .bind(&input.customer_name)
        .bind(&input.billing_line1)
        .bind(&input.billing_line2)
        .bind(&input.billing_city)
        .bind(&input.billing_state)
        .bind(&input.billing_postal_code)
        .bind(&input.billing_country)
        .bind(&input.currency)
        .bind(input.due_date)
        .bind(&input.notes)
        .bind(input.reference_invoice_id)
        .bind(&input.metadata)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to create invoice: {}", e)))?;

        timer.observe_duration();

        info!(invoice_id = %invoice.invoice_id, "Draft invoice created");

        Ok(invoice)
    }

    /// Get an invoice by ID.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn get_invoice(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Option<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_invoice"])
            .start_timer();

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            SELECT invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            FROM invoices
            WHERE tenant_id = $1 AND invoice_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get invoice: {}", e)))?;

        timer.observe_duration();

        Ok(invoice)
    }

    /// List invoices for a tenant.
    #[instrument(skip(self, filter), fields(tenant_id = %tenant_id))]
    pub async fn list_invoices(
        &self,
        tenant_id: Uuid,
        filter: &ListInvoicesFilter,
    ) -> Result<Vec<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_invoices"])
            .start_timer();

        let limit = filter.page_size.clamp(1, 100) as i64;
        let status_str = filter.status.map(|s| s.as_str().to_string());

        let invoices = if let Some(cursor) = filter.page_token {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                    billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                    currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                    notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
                FROM invoices
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR status = $2)
                  AND ($3::uuid IS NULL OR customer_id = $3)
                  AND ($4::date IS NULL OR issue_date >= $4)
                  AND ($5::date IS NULL OR issue_date <= $5)
                  AND invoice_id > $6
                ORDER BY invoice_id
                LIMIT $7
                "#,
            )
            .bind(tenant_id)
            .bind(&status_str)
            .bind(filter.customer_id)
            .bind(filter.start_date)
            .bind(filter.end_date)
            .bind(cursor)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                    billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                    currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                    notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
                FROM invoices
                WHERE tenant_id = $1
                  AND ($2::varchar IS NULL OR status = $2)
                  AND ($3::uuid IS NULL OR customer_id = $3)
                  AND ($4::date IS NULL OR issue_date >= $4)
                  AND ($5::date IS NULL OR issue_date <= $5)
                ORDER BY invoice_id
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(&status_str)
            .bind(filter.customer_id)
            .bind(filter.start_date)
            .bind(filter.end_date)
            .bind(limit)
            .fetch_all(self.pool())
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list invoices: {}", e)))?;

        timer.observe_duration();

        Ok(invoices)
    }

    /// Issue an invoice (assign number, set status to issued).
    #[instrument(skip(self), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn issue_invoice(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
        issue_date: NaiveDate,
        journal_id: Option<Uuid>,
    ) -> Result<Option<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["issue_invoice"])
            .start_timer();

        // First check if invoice is in draft status
        let existing = self.get_invoice(tenant_id, invoice_id).await?;
        match existing {
            Some(inv) if inv.status == "draft" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Only draft invoices can be issued"
                )))
            }
            None => return Ok(None),
        };

        // Check if invoice has line items
        let line_items = self.get_line_items(tenant_id, invoice_id).await?;
        if line_items.is_empty() {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Cannot issue invoice without line items"
            )));
        }

        // Generate invoice number and issue
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET invoice_number = next_invoice_number($1),
                status = 'issued',
                issue_date = $3,
                issued_utc = NOW(),
                journal_id = $4,
                amount_due = total
            WHERE tenant_id = $1 AND invoice_id = $2 AND status = 'draft'
            RETURNING invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(issue_date)
        .bind(journal_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to issue invoice: {}", e)))?;

        timer.observe_duration();

        if let Some(ref inv) = invoice {
            info!(
                invoice_id = %inv.invoice_id,
                invoice_number = %inv.invoice_number.as_deref().unwrap_or(""),
                "Invoice issued"
            );
        }

        Ok(invoice)
    }

    /// Void an invoice.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn void_invoice(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Option<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["void_invoice"])
            .start_timer();

        // Check if invoice is in issued status
        let existing = self.get_invoice(tenant_id, invoice_id).await?;
        match existing {
            Some(inv) if inv.status == "issued" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Only issued invoices can be voided"
                )))
            }
            None => return Ok(None),
        };

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET status = 'void',
                voided_utc = NOW()
            WHERE tenant_id = $1 AND invoice_id = $2 AND status = 'issued'
            RETURNING invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to void invoice: {}", e)))?;

        timer.observe_duration();

        if let Some(ref inv) = invoice {
            info!(invoice_id = %inv.invoice_id, "Invoice voided");
        }

        Ok(invoice)
    }

    /// Delete a draft invoice.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn delete_invoice(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<bool, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["delete_invoice"])
            .start_timer();

        let result = sqlx::query(
            r#"
            DELETE FROM invoices
            WHERE tenant_id = $1 AND invoice_id = $2 AND status = 'draft'
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to delete invoice: {}", e)))?;

        timer.observe_duration();

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!(invoice_id = %invoice_id, "Draft invoice deleted");
        }

        Ok(deleted)
    }

    /// Update a draft invoice.
    #[instrument(skip(self, input), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn update_invoice(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
        input: &UpdateInvoice,
    ) -> Result<Option<Invoice>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_invoice"])
            .start_timer();

        // First check if invoice is in draft status
        let existing = self.get_invoice(tenant_id, invoice_id).await?;
        match existing {
            Some(inv) if inv.status == "draft" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Only draft invoices can be updated"
                )))
            }
            None => return Ok(None),
        };

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET customer_name = COALESCE($3, customer_name),
                billing_line1 = COALESCE($4, billing_line1),
                billing_line2 = COALESCE($5, billing_line2),
                billing_city = COALESCE($6, billing_city),
                billing_state = COALESCE($7, billing_state),
                billing_postal_code = COALESCE($8, billing_postal_code),
                billing_country = COALESCE($9, billing_country),
                due_date = COALESCE($10, due_date),
                notes = COALESCE($11, notes),
                metadata = COALESCE($12, metadata)
            WHERE tenant_id = $1 AND invoice_id = $2 AND status = 'draft'
            RETURNING invoice_id, tenant_id, invoice_number, invoice_type, status, customer_id, customer_name,
                billing_line1, billing_line2, billing_city, billing_state, billing_postal_code, billing_country,
                currency, issue_date, due_date, subtotal, tax_total, total, amount_paid, amount_due,
                notes, reference_invoice_id, journal_id, metadata, created_utc, issued_utc, voided_utc
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(&input.customer_name)
        .bind(&input.billing_line1)
        .bind(&input.billing_line2)
        .bind(&input.billing_city)
        .bind(&input.billing_state)
        .bind(&input.billing_postal_code)
        .bind(&input.billing_country)
        .bind(input.due_date)
        .bind(&input.notes)
        .bind(&input.metadata)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update invoice: {}", e)))?;

        timer.observe_duration();

        if let Some(ref inv) = invoice {
            info!(invoice_id = %inv.invoice_id, "Invoice updated");
        }

        Ok(invoice)
    }

    // -------------------------------------------------------------------------
    // Line Item Operations
    // -------------------------------------------------------------------------

    /// Add a line item to an invoice.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id, invoice_id = %input.invoice_id))]
    pub async fn add_line_item(&self, input: &CreateLineItem) -> Result<LineItem, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["add_line_item"])
            .start_timer();

        // Verify invoice is in draft status
        let invoice = self.get_invoice(input.tenant_id, input.invoice_id).await?;
        match invoice {
            Some(inv) if inv.status == "draft" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Can only add line items to draft invoices"
                )))
            }
            None => {
                return Err(AppError::NotFound(anyhow::anyhow!("Invoice not found")));
            }
        };

        // Calculate amounts
        let subtotal = input.quantity * input.unit_price;
        let tax_amount = if let Some(tax_rate_id) = input.tax_rate_id {
            let tax_rate = self.get_tax_rate(input.tenant_id, tax_rate_id).await?;
            if let Some(rate) = tax_rate {
                if rate.calculation == "inclusive" {
                    subtotal - (subtotal / (Decimal::ONE + rate.rate))
                } else {
                    subtotal * rate.rate
                }
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };
        let total = subtotal + tax_amount;

        let line_item_id = Uuid::new_v4();
        let line_item = sqlx::query_as::<_, LineItem>(
            r#"
            INSERT INTO line_items (
                line_item_id, invoice_id, tenant_id, description, quantity, unit_price,
                tax_rate_id, tax_amount, subtotal, total, ledger_account_id, sort_order
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING line_item_id, invoice_id, tenant_id, description, quantity, unit_price,
                tax_rate_id, tax_amount, subtotal, total, ledger_account_id, sort_order, created_utc
            "#,
        )
        .bind(line_item_id)
        .bind(input.invoice_id)
        .bind(input.tenant_id)
        .bind(&input.description)
        .bind(input.quantity)
        .bind(input.unit_price)
        .bind(input.tax_rate_id)
        .bind(tax_amount)
        .bind(subtotal)
        .bind(total)
        .bind(input.ledger_account_id)
        .bind(input.sort_order)
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to add line item: {}", e)))?;

        timer.observe_duration();

        info!(line_item_id = %line_item.line_item_id, "Line item added");

        Ok(line_item)
    }

    /// Get line items for an invoice.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, invoice_id = %invoice_id))]
    pub async fn get_line_items(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
    ) -> Result<Vec<LineItem>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_line_items"])
            .start_timer();

        let line_items = sqlx::query_as::<_, LineItem>(
            r#"
            SELECT line_item_id, invoice_id, tenant_id, description, quantity, unit_price,
                tax_rate_id, tax_amount, subtotal, total, ledger_account_id, sort_order, created_utc
            FROM line_items
            WHERE tenant_id = $1 AND invoice_id = $2
            ORDER BY sort_order, created_utc
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get line items: {}", e)))?;

        timer.observe_duration();

        Ok(line_items)
    }

    /// Update a line item.
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self, input), fields(tenant_id = %tenant_id, line_item_id = %line_item_id))]
    pub async fn update_line_item(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
        line_item_id: Uuid,
        input: &UpdateLineItem,
    ) -> Result<Option<LineItem>, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_line_item"])
            .start_timer();

        // Verify invoice is in draft status
        let invoice = self.get_invoice(tenant_id, invoice_id).await?;
        match invoice {
            Some(inv) if inv.status == "draft" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Can only update line items on draft invoices"
                )))
            }
            None => return Ok(None),
        };

        // Calculate new amounts if quantity or price changed
        let quantity = input.quantity.unwrap_or(Decimal::ZERO);
        let unit_price = input.unit_price.unwrap_or(Decimal::ZERO);
        let subtotal = quantity * unit_price;

        let tax_amount = if let Some(tax_rate_id) = input.tax_rate_id {
            let tax_rate = self.get_tax_rate(tenant_id, tax_rate_id).await?;
            if let Some(rate) = tax_rate {
                if rate.calculation == "inclusive" {
                    subtotal - (subtotal / (Decimal::ONE + rate.rate))
                } else {
                    subtotal * rate.rate
                }
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };
        let total = subtotal + tax_amount;

        let line_item = sqlx::query_as::<_, LineItem>(
            r#"
            UPDATE line_items
            SET description = COALESCE($4, description),
                quantity = COALESCE($5, quantity),
                unit_price = COALESCE($6, unit_price),
                tax_rate_id = $7,
                tax_amount = $8,
                subtotal = $9,
                total = $10,
                ledger_account_id = $11,
                sort_order = COALESCE($12, sort_order)
            WHERE tenant_id = $1 AND invoice_id = $2 AND line_item_id = $3
            RETURNING line_item_id, invoice_id, tenant_id, description, quantity, unit_price,
                tax_rate_id, tax_amount, subtotal, total, ledger_account_id, sort_order, created_utc
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(line_item_id)
        .bind(&input.description)
        .bind(input.quantity)
        .bind(input.unit_price)
        .bind(input.tax_rate_id)
        .bind(tax_amount)
        .bind(subtotal)
        .bind(total)
        .bind(input.ledger_account_id)
        .bind(input.sort_order)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to update line item: {}", e))
        })?;

        timer.observe_duration();

        Ok(line_item)
    }

    /// Remove a line item.
    #[instrument(skip(self), fields(tenant_id = %tenant_id, line_item_id = %line_item_id))]
    pub async fn remove_line_item(
        &self,
        tenant_id: Uuid,
        invoice_id: Uuid,
        line_item_id: Uuid,
    ) -> Result<bool, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["remove_line_item"])
            .start_timer();

        // Verify invoice is in draft status
        let invoice = self.get_invoice(tenant_id, invoice_id).await?;
        match invoice {
            Some(inv) if inv.status == "draft" => {}
            Some(_) => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Can only remove line items from draft invoices"
                )))
            }
            None => return Ok(false),
        };

        let result = sqlx::query(
            r#"
            DELETE FROM line_items
            WHERE tenant_id = $1 AND invoice_id = $2 AND line_item_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(invoice_id)
        .bind(line_item_id)
        .execute(self.pool())
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to remove line item: {}", e))
        })?;

        timer.observe_duration();

        Ok(result.rows_affected() > 0)
    }
}
