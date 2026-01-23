//! Database service for invoicing-service.

use crate::models::{
    CreateInvoice, CreateLineItem, CreateReceipt, CreateTaxRate, Invoice, LineItem,
    ListInvoicesFilter, ListReceiptsFilter, Receipt, TaxRate, UpdateInvoice, UpdateLineItem,
    UpdateTaxRate,
};
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
    #[instrument(skip(database_url), fields(service = "invoicing-service"))]
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
    // Tax Rate Operations
    // -------------------------------------------------------------------------

    /// Create a new tax rate.
    #[instrument(skip(self, input), fields(tenant_id = %input.tenant_id))]
    pub async fn create_tax_rate(&self, input: &CreateTaxRate) -> Result<TaxRate, AppError> {
        let timer = DB_QUERY_DURATION
            .with_label_values(&["create_tax_rate"])
            .start_timer();

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
        .fetch_one(&self.pool)
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

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["get_tax_rate"])
            .start_timer();

        let tax_rate = sqlx::query_as::<_, TaxRate>(
            r#"
            SELECT tax_rate_id, tenant_id, name, rate, calculation, effective_from, effective_to, active, created_utc
            FROM tax_rates
            WHERE tenant_id = $1 AND tax_rate_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(tax_rate_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get tax rate: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["list_tax_rates"])
            .start_timer();

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to list tax rates: {}", e)))?;

        timer.observe_duration();

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
        let timer = DB_QUERY_DURATION
            .with_label_values(&["update_tax_rate"])
            .start_timer();

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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to update tax rate: {}", e)))?;

        timer.observe_duration();

        Ok(tax_rate)
    }

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
        .fetch_one(&self.pool)
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
        .fetch_optional(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .execute(&self.pool)
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
        .fetch_optional(&self.pool)
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
        .fetch_one(&self.pool)
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!("Failed to get line items: {}", e)))?;

        timer.observe_duration();

        Ok(line_items)
    }

    /// Update a line item.
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
        .fetch_optional(&self.pool)
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
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to remove line item: {}", e))
        })?;

        timer.observe_duration();

        Ok(result.rows_affected() > 0)
    }

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
        .fetch_one(&self.pool)
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
        .fetch_optional(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
        .fetch_one(&self.pool)
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
        .fetch_one(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            AppError::DatabaseError(anyhow::anyhow!("Failed to get customer info: {}", e))
        })?;

        Ok(invoice)
    }
}
