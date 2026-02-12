//! Statement gRPC handler implementations.

use crate::grpc::helpers::{datetime_to_timestamp, format_decimal};
use crate::grpc::proto::{
    Address, GenerateStatementRequest, GenerateStatementResponse, Statement as ProtoStatement,
    StatementLine as ProtoStatementLine,
};
use crate::grpc::service::InvoicingServiceImpl;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateStatement",
            tenant_id,
            customer_id
        )
    )]
    pub(crate) async fn handle_generate_statement(
        &self,
        request: Request<GenerateStatementRequest>,
    ) -> Result<Response<GenerateStatementResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
            Status::invalid_argument("Invalid customer_id format")
        })?;
        Span::current().record("customer_id", customer_id.to_string());

        let period_start =
            NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid period_start format")
            })?;

        let period_end = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d").map_err(|_| {
            Status::invalid_argument("Invalid period_end format")
        })?;

        if period_start > period_end {
            return Err(Status::invalid_argument(
                "period_start must be before period_end",
            ));
        }

        // Get customer info from most recent invoice
        let customer_info = self.db.get_customer_info(tenant_id, customer_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get customer info");
            Status::internal("Failed to get customer info")
        })?;

        let (customer_name, billing_address, currency) = match customer_info {
            Some(inv) => (
                inv.customer_name,
                Some(Address {
                    line1: inv.billing_line1.unwrap_or_default(),
                    line2: inv.billing_line2.unwrap_or_default(),
                    city: inv.billing_city.unwrap_or_default(),
                    state: inv.billing_state.unwrap_or_default(),
                    postal_code: inv.billing_postal_code.unwrap_or_default(),
                    country: inv.billing_country.unwrap_or_default(),
                }),
                inv.currency,
            ),
            None => {
                return Err(Status::not_found("No invoices found for customer"));
            }
        };

        // Calculate opening balance
        let opening_balance = self.db.calculate_opening_balance(tenant_id, customer_id, period_start).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to calculate opening balance");
            Status::internal("Failed to calculate opening balance")
        })?;

        // Get invoices in period
        let invoices = self.db.get_invoices_for_statement(tenant_id, customer_id, period_start, period_end).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get invoices for statement");
            Status::internal("Failed to get invoices")
        })?;

        // Get receipts in period
        let receipts = self.db.get_receipts_for_statement(tenant_id, customer_id, period_start, period_end).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get receipts for statement");
            Status::internal("Failed to get receipts")
        })?;

        // Build statement lines and calculate totals
        let mut lines: Vec<(NaiveDate, ProtoStatementLine)> = Vec::new();
        let mut total_debits = Decimal::ZERO;
        let mut total_credits = Decimal::ZERO;

        // Add invoice lines (debits)
        for inv in &invoices {
            let issue_date = inv.issue_date.unwrap_or(inv.created_utc.date_naive());
            let doc_type = if inv.invoice_type == "credit_note" {
                "credit_note"
            } else {
                "invoice"
            };
            let (debit, credit) = if inv.invoice_type == "credit_note" {
                total_credits += inv.total;
                (Decimal::ZERO, inv.total)
            } else {
                total_debits += inv.total;
                (inv.total, Decimal::ZERO)
            };

            lines.push((
                issue_date,
                ProtoStatementLine {
                    date: issue_date.format("%Y-%m-%d").to_string(),
                    document_type: doc_type.to_string(),
                    document_number: inv.invoice_number.clone().unwrap_or_default(),
                    description: format!(
                        "Invoice {}",
                        inv.invoice_number
                            .as_deref()
                            .unwrap_or(&inv.invoice_id.to_string())
                    ),
                    debit: format_decimal(&debit),
                    credit: format_decimal(&credit),
                    balance: String::new(), // Will be calculated below
                },
            ));
        }

        // Add receipt lines (credits)
        for receipt in &receipts {
            total_credits += receipt.amount;
            lines.push((
                receipt.payment_date,
                ProtoStatementLine {
                    date: receipt.payment_date.format("%Y-%m-%d").to_string(),
                    document_type: "payment".to_string(),
                    document_number: receipt.receipt_number.clone(),
                    description: format!("Payment - {}", receipt.payment_method),
                    debit: format_decimal(&Decimal::ZERO),
                    credit: format_decimal(&receipt.amount),
                    balance: String::new(), // Will be calculated below
                },
            ));
        }

        // Sort by date
        lines.sort_by(|a, b| a.0.cmp(&b.0));

        // Calculate running balance
        let mut running_balance = opening_balance;
        let statement_lines: Vec<ProtoStatementLine> = lines
            .into_iter()
            .map(|(_, mut line)| {
                let debit: Decimal = line.debit.parse().unwrap_or(Decimal::ZERO);
                let credit: Decimal = line.credit.parse().unwrap_or(Decimal::ZERO);
                running_balance = running_balance + debit - credit;
                line.balance = format_decimal(&running_balance);
                line
            })
            .collect();

        let closing_balance = running_balance;

        info!(
            tenant_id = %tenant_id,
            customer_id = %customer_id,
            period_start = %period_start,
            period_end = %period_end,
            opening_balance = %opening_balance,
            closing_balance = %closing_balance,
            lines = statement_lines.len(),
            "Statement generated"
        );

        Ok(Response::new(GenerateStatementResponse {
            statement: Some(ProtoStatement {
                tenant_id: tenant_id.to_string(),
                customer_id: customer_id.to_string(),
                customer_name,
                billing_address,
                currency,
                period_start: period_start.format("%Y-%m-%d").to_string(),
                period_end: period_end.format("%Y-%m-%d").to_string(),
                opening_balance: format_decimal(&opening_balance),
                closing_balance: format_decimal(&closing_balance),
                total_debits: format_decimal(&total_debits),
                total_credits: format_decimal(&total_credits),
                lines: statement_lines,
                generated_at: Some(datetime_to_timestamp(chrono::Utc::now())),
            }),
        }))
    }
}
