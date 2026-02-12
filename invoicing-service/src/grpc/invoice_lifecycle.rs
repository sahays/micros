//! Invoice lifecycle gRPC handler implementations (issue, void).

use crate::grpc::helpers::{format_decimal, invoice_to_proto};
use crate::grpc::proto::{
    IssueInvoiceRequest, IssueInvoiceResponse, VoidInvoiceRequest, VoidInvoiceResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use chrono::NaiveDate;
use service_core::grpc::TransactionEntry;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "IssueInvoice",
            tenant_id,
            invoice_id
        )
    )]
    pub(crate) async fn handle_issue_invoice(
        &self,
        request: Request<IssueInvoiceRequest>,
    ) -> Result<Response<IssueInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        let issue_date = if req.issue_date.is_empty() {
            chrono::Utc::now().date_naive()
        } else {
            NaiveDate::parse_from_str(&req.issue_date, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid issue_date format")
            })?
        };

        // Get invoice and line items for ledger entry
        let existing_invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice for ledger entry");
                Status::internal("Failed to get invoice")
            })?;
        let existing_invoice = existing_invoice.ok_or_else(|| {
            Status::not_found("Invoice not found")
        })?;

        let line_items = self
            .db
            .get_line_items(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get line items for ledger entry");
                Status::internal("Failed to get line items")
            })?;

        // Create ledger entry if ledger client is available
        let journal_id = if let Some(ref ledger_client) = self.ledger_client {
            // Build ledger entries: Debit A/R, Credit Revenue accounts
            // Convention: A/R account = "AR-{currency}", Revenue from line item ledger_account_id
            let ar_account_id = format!("AR-{}", existing_invoice.currency);
            let total_str = format_decimal(&existing_invoice.total);
            let idempotency_key = format!("invoice-issue-{}", invoice_id);

            let mut entries = vec![TransactionEntry::debit(&ar_account_id, &total_str)];

            // Credit revenue accounts based on line items
            for item in &line_items {
                let revenue_account = item
                    .ledger_account_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| format!("REVENUE-{}", existing_invoice.currency));
                entries.push(TransactionEntry::credit(
                    &revenue_account,
                    &format_decimal(&item.total),
                ));
            }

            let metadata = serde_json::json!({
                "source": "invoicing-service",
                "invoice_id": invoice_id.to_string(),
                "customer_id": existing_invoice.customer_id.to_string(),
            })
            .to_string();

            match ledger_client
                .post_transaction(
                    &tenant_id.to_string(),
                    entries,
                    Some(&issue_date.to_string()),
                    &idempotency_key,
                    Some(&metadata),
                )
                .await
            {
                Ok(response) => {
                    if let Some(ref txn) = response.transaction {
                        info!(journal_id = %txn.journal_id, "Ledger entry created for invoice issue");
                        Uuid::parse_str(&txn.journal_id).ok()
                    } else {
                        warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, "Ledger response missing transaction");
                        None
                    }
                }
                Err(e) => {
                    // Log but don't fail - ledger integration is optional enhancement
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to create ledger entry for invoice issue");
                    None
                }
            }
        } else {
            None
        };

        let invoice = self.db.issue_invoice(tenant_id, invoice_id, issue_date, journal_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to issue invoice");
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to issue invoice"),
            }
        })?;

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    Status::internal("Failed to get line items")
                })?;
                info!(
                    tenant_id = %tenant_id,
                    invoice_id = %inv.invoice_id,
                    invoice_number = %inv.invoice_number.as_deref().unwrap_or(""),
                    total = %inv.total,
                    currency = %inv.currency,
                    "Invoice issued"
                );
                Ok(Response::new(IssueInvoiceResponse {
                    invoice: Some(invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                Err(Status::not_found("Invoice not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "VoidInvoice",
            tenant_id,
            invoice_id
        )
    )]
    pub(crate) async fn handle_void_invoice(
        &self,
        request: Request<VoidInvoiceRequest>,
    ) -> Result<Response<VoidInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        // Get invoice for reversing ledger entry
        let existing_invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice for reversing entry");
                Status::internal("Failed to get invoice")
            })?;

        if let Some(ref inv) = existing_invoice {
            // Create reversing ledger entry if ledger client is available
            if let Some(ref ledger_client) = self.ledger_client {
                let line_items = self
                    .db
                    .get_line_items(tenant_id, invoice_id)
                    .await
                    .map_err(|e| {
                        warn!(error = %e, "Failed to get line items for reversing entry");
                        Status::internal("Failed to get line items")
                    })?;

                // Reverse the original entry: Credit A/R, Debit Revenue
                let ar_account_id = format!("AR-{}", inv.currency);
                let total_str = format_decimal(&inv.total);
                let idempotency_key = format!("invoice-void-{}", invoice_id);

                let mut entries = vec![TransactionEntry::credit(&ar_account_id, &total_str)];

                // Debit revenue accounts (reversal)
                for item in &line_items {
                    let revenue_account = item
                        .ledger_account_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| format!("REVENUE-{}", inv.currency));
                    entries.push(TransactionEntry::debit(
                        &revenue_account,
                        &format_decimal(&item.total),
                    ));
                }

                let metadata = serde_json::json!({
                    "source": "invoicing-service",
                    "invoice_id": invoice_id.to_string(),
                    "action": "void",
                    "original_journal_id": inv.journal_id.map(|j| j.to_string()),
                })
                .to_string();

                if let Err(e) = ledger_client
                    .post_transaction(
                        &tenant_id.to_string(),
                        entries,
                        Some(&chrono::Utc::now().date_naive().to_string()),
                        &idempotency_key,
                        Some(&metadata),
                    )
                    .await
                {
                    // Log but don't fail - ledger integration is optional enhancement
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to create reversing ledger entry");
                } else {
                    info!(invoice_id = %invoice_id, "Reversing ledger entry created for voided invoice");
                }
            }
        }

        let invoice = self.db.void_invoice(tenant_id, invoice_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to void invoice");
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to void invoice"),
            }
        })?;

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    Status::internal("Failed to get line items")
                })?;
                info!(tenant_id = %tenant_id, invoice_id = %inv.invoice_id, "Invoice voided");
                Ok(Response::new(VoidInvoiceResponse {
                    invoice: Some(invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                Err(Status::not_found("Invoice not found"))
            }
        }
    }
}
