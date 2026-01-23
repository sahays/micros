//! InvoicingService gRPC implementation.

use crate::grpc::proto::{
    invoicing_service_server::InvoicingService, AddLineItemRequest, AddLineItemResponse, Address,
    CreateInvoiceRequest, CreateInvoiceResponse, CreateTaxRateRequest, CreateTaxRateResponse,
    DeleteInvoiceRequest, DeleteInvoiceResponse, GenerateInvoicePdfRequest,
    GenerateInvoicePdfResponse, GenerateReceiptPdfRequest, GenerateReceiptPdfResponse,
    GenerateStatementPdfRequest, GenerateStatementPdfResponse, GenerateStatementRequest,
    GenerateStatementResponse, GetInvoiceRequest, GetInvoiceResponse, GetReceiptRequest,
    GetReceiptResponse, GetTaxRateRequest, GetTaxRateResponse, Invoice as ProtoInvoice,
    InvoiceStatus as ProtoInvoiceStatus, InvoiceType as ProtoInvoiceType, IssueInvoiceRequest,
    IssueInvoiceResponse, LineItem as ProtoLineItem, ListInvoicesRequest, ListInvoicesResponse,
    ListReceiptsRequest, ListReceiptsResponse, ListTaxRatesRequest, ListTaxRatesResponse,
    Receipt as ProtoReceipt, RecordPaymentRequest, RecordPaymentResponse, RemoveLineItemRequest,
    RemoveLineItemResponse, Statement as ProtoStatement, StatementLine as ProtoStatementLine,
    TaxCalculation, TaxRate as ProtoTaxRate, UpdateInvoiceRequest, UpdateInvoiceResponse,
    UpdateLineItemRequest, UpdateLineItemResponse, UpdateTaxRateRequest, UpdateTaxRateResponse,
    VoidInvoiceRequest, VoidInvoiceResponse,
};
use crate::models::{
    CreateInvoice, CreateLineItem, CreateReceipt, CreateTaxRate, Invoice, InvoiceStatus, LineItem,
    ListInvoicesFilter, ListReceiptsFilter, Receipt, TaxRate, UpdateInvoice, UpdateLineItem,
    UpdateTaxRate,
};
use crate::services::metrics::{
    ERRORS_TOTAL, GRPC_REQUESTS_TOTAL, GRPC_REQUEST_DURATION, INVOICES_TOTAL, INVOICE_AMOUNT_TOTAL,
    PAYMENT_AMOUNT_TOTAL, RECEIPTS_TOTAL,
};
use crate::services::Database;
use chrono::NaiveDate;
use prost_types::Timestamp;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use service_core::grpc::{LedgerClient, TransactionEntry};
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

/// Format a Decimal as a normalized string.
fn format_decimal(d: &Decimal) -> String {
    let s = d.to_string();
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// InvoicingService implementation.
pub struct InvoicingServiceImpl {
    db: Arc<Database>,
    ledger_client: Option<Arc<LedgerClient>>,
}

impl InvoicingServiceImpl {
    /// Create a new InvoicingService instance.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            ledger_client: None,
        }
    }

    /// Create a new InvoicingService instance with a ledger client.
    pub fn with_ledger_client(db: Arc<Database>, ledger_client: Arc<LedgerClient>) -> Self {
        Self {
            db,
            ledger_client: Some(ledger_client),
        }
    }

    /// Convert domain TaxRate to proto TaxRate.
    fn tax_rate_to_proto(rate: &TaxRate) -> ProtoTaxRate {
        ProtoTaxRate {
            tax_rate_id: rate.tax_rate_id.to_string(),
            tenant_id: rate.tenant_id.to_string(),
            name: rate.name.clone(),
            rate: format_decimal(&rate.rate),
            calculation: match rate.calculation.as_str() {
                "inclusive" => TaxCalculation::Inclusive as i32,
                _ => TaxCalculation::Exclusive as i32,
            },
            effective_from: rate.effective_from.to_string(),
            effective_to: rate.effective_to.map(|d| d.to_string()).unwrap_or_default(),
            active: rate.active,
            created_at: Some(Timestamp {
                seconds: rate.created_utc.timestamp(),
                nanos: rate.created_utc.timestamp_subsec_nanos() as i32,
            }),
        }
    }

    /// Convert domain LineItem to proto LineItem.
    fn line_item_to_proto(item: &LineItem) -> ProtoLineItem {
        ProtoLineItem {
            line_item_id: item.line_item_id.to_string(),
            invoice_id: item.invoice_id.to_string(),
            description: item.description.clone(),
            quantity: format_decimal(&item.quantity),
            unit_price: format_decimal(&item.unit_price),
            tax_rate_id: item
                .tax_rate_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            tax_amount: format_decimal(&item.tax_amount),
            subtotal: format_decimal(&item.subtotal),
            total: format_decimal(&item.total),
            ledger_account_id: item
                .ledger_account_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            sort_order: item.sort_order,
        }
    }

    /// Compute the effective status of an invoice, checking for overdue condition.
    fn compute_invoice_status(invoice: &Invoice) -> i32 {
        // If invoice is issued and has a due date that's past, it's overdue
        if invoice.status == "issued" {
            if let Some(due_date) = invoice.due_date {
                let today = chrono::Utc::now().date_naive();
                if due_date < today && invoice.amount_due > Decimal::ZERO {
                    return ProtoInvoiceStatus::Overdue as i32;
                }
            }
            return ProtoInvoiceStatus::Issued as i32;
        }

        match invoice.status.as_str() {
            "paid" => ProtoInvoiceStatus::Paid as i32,
            "void" => ProtoInvoiceStatus::Void as i32,
            "overdue" => ProtoInvoiceStatus::Overdue as i32,
            _ => ProtoInvoiceStatus::Draft as i32,
        }
    }

    /// Convert domain Invoice to proto Invoice.
    fn invoice_to_proto(invoice: &Invoice, line_items: &[LineItem]) -> ProtoInvoice {
        ProtoInvoice {
            invoice_id: invoice.invoice_id.to_string(),
            tenant_id: invoice.tenant_id.to_string(),
            invoice_number: invoice.invoice_number.clone().unwrap_or_default(),
            invoice_type: match invoice.invoice_type.as_str() {
                "credit_note" => ProtoInvoiceType::CreditNote as i32,
                "proforma" => ProtoInvoiceType::Proforma as i32,
                _ => ProtoInvoiceType::Standard as i32,
            },
            status: Self::compute_invoice_status(invoice),
            customer_id: invoice.customer_id.to_string(),
            customer_name: invoice.customer_name.clone(),
            billing_address: Some(Address {
                line1: invoice.billing_line1.clone().unwrap_or_default(),
                line2: invoice.billing_line2.clone().unwrap_or_default(),
                city: invoice.billing_city.clone().unwrap_or_default(),
                state: invoice.billing_state.clone().unwrap_or_default(),
                postal_code: invoice.billing_postal_code.clone().unwrap_or_default(),
                country: invoice.billing_country.clone().unwrap_or_default(),
            }),
            currency: invoice.currency.clone(),
            issue_date: invoice
                .issue_date
                .map(|d| d.to_string())
                .unwrap_or_default(),
            due_date: invoice.due_date.map(|d| d.to_string()).unwrap_or_default(),
            line_items: line_items.iter().map(Self::line_item_to_proto).collect(),
            subtotal: format_decimal(&invoice.subtotal),
            tax_total: format_decimal(&invoice.tax_total),
            total: format_decimal(&invoice.total),
            amount_paid: format_decimal(&invoice.amount_paid),
            amount_due: format_decimal(&invoice.amount_due),
            notes: invoice.notes.clone().unwrap_or_default(),
            reference_invoice_id: invoice
                .reference_invoice_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            journal_id: invoice
                .journal_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            metadata: invoice
                .metadata
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_default(),
            created_at: Some(Timestamp {
                seconds: invoice.created_utc.timestamp(),
                nanos: invoice.created_utc.timestamp_subsec_nanos() as i32,
            }),
            issued_at: invoice.issued_utc.map(|t| Timestamp {
                seconds: t.timestamp(),
                nanos: t.timestamp_subsec_nanos() as i32,
            }),
            voided_at: invoice.voided_utc.map(|t| Timestamp {
                seconds: t.timestamp(),
                nanos: t.timestamp_subsec_nanos() as i32,
            }),
        }
    }

    /// Convert domain Receipt to proto Receipt.
    fn receipt_to_proto(receipt: &Receipt) -> ProtoReceipt {
        ProtoReceipt {
            receipt_id: receipt.receipt_id.to_string(),
            tenant_id: receipt.tenant_id.to_string(),
            receipt_number: receipt.receipt_number.clone(),
            invoice_id: receipt.invoice_id.to_string(),
            customer_id: receipt.customer_id.to_string(),
            amount: format_decimal(&receipt.amount),
            currency: receipt.currency.clone(),
            payment_method: receipt.payment_method.clone(),
            payment_reference: receipt.payment_reference.clone().unwrap_or_default(),
            payment_date: receipt.payment_date.to_string(),
            journal_id: receipt
                .journal_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            notes: receipt.notes.clone().unwrap_or_default(),
            created_at: Some(Timestamp {
                seconds: receipt.created_utc.timestamp(),
                nanos: receipt.created_utc.timestamp_subsec_nanos() as i32,
            }),
        }
    }

    /// Convert DateTime to proto Timestamp.
    fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}

#[tonic::async_trait]
impl InvoicingService for InvoicingServiceImpl {
    // -------------------------------------------------------------------------
    // Tax Rate Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "CreateTaxRate",
            tenant_id,
            tax_rate_id
        )
    )]
    async fn create_tax_rate(
        &self,
        request: Request<CreateTaxRateRequest>,
    ) -> Result<Response<CreateTaxRateResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["CreateTaxRate"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateTaxRate", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let rate = Decimal::from_str(&req.rate).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateTaxRate", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid rate format")
        })?;

        let effective_from =
            NaiveDate::parse_from_str(&req.effective_from, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["CreateTaxRate", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid effective_from format")
            })?;

        let effective_to = if req.effective_to.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_to, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["CreateTaxRate", "invalid_argument"])
                        .inc();
                    ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                    Status::invalid_argument("Invalid effective_to format")
                })?,
            )
        };

        let calculation = match req.calculation {
            x if x == TaxCalculation::Inclusive as i32 => "inclusive",
            _ => "exclusive",
        };

        let input = CreateTaxRate {
            tenant_id,
            name: req.name,
            rate,
            calculation: calculation.to_string(),
            effective_from,
            effective_to,
        };

        let tax_rate = self.db.create_tax_rate(&input).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, error = %e, "Failed to create tax rate");
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateTaxRate", "error"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to create tax rate")
        })?;

        Span::current().record("tax_rate_id", tax_rate.tax_rate_id.to_string());
        GRPC_REQUESTS_TOTAL
            .with_label_values(&["CreateTaxRate", "ok"])
            .inc();
        timer.observe_duration();

        info!(tenant_id = %tenant_id, tax_rate_id = %tax_rate.tax_rate_id, "Tax rate created");

        Ok(Response::new(CreateTaxRateResponse {
            tax_rate: Some(Self::tax_rate_to_proto(&tax_rate)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetTaxRate")
    )]
    async fn get_tax_rate(
        &self,
        request: Request<GetTaxRateRequest>,
    ) -> Result<Response<GetTaxRateResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetTaxRate"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetTaxRate", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let tax_rate_id = Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetTaxRate", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tax_rate_id format")
        })?;

        let tax_rate = self
            .db
            .get_tax_rate(tenant_id, tax_rate_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get tax rate");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetTaxRate", "error"])
                    .inc();
                Status::internal("Failed to get tax rate")
            })?;

        timer.observe_duration();

        match tax_rate {
            Some(rate) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetTaxRate", "ok"])
                    .inc();
                Ok(Response::new(GetTaxRateResponse {
                    tax_rate: Some(Self::tax_rate_to_proto(&rate)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetTaxRate", "not_found"])
                    .inc();
                Err(Status::not_found("Tax rate not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "ListTaxRates")
    )]
    async fn list_tax_rates(
        &self,
        request: Request<ListTaxRatesRequest>,
    ) -> Result<Response<ListTaxRatesResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["ListTaxRates"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["ListTaxRates", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let as_of_date = if req.as_of_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListTaxRates", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid as_of_date format")
                })?,
            )
        };

        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListTaxRates", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid page_token format")
            })?)
        };

        let page_size = if req.page_size <= 0 {
            20
        } else {
            req.page_size
        };

        let tax_rates = self
            .db
            .list_tax_rates(
                tenant_id,
                req.active_only,
                as_of_date,
                page_size,
                page_token,
            )
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to list tax rates");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListTaxRates", "error"])
                    .inc();
                Status::internal("Failed to list tax rates")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["ListTaxRates", "ok"])
            .inc();
        timer.observe_duration();

        let next_page_token = if tax_rates.len() == page_size as usize {
            tax_rates.last().map(|r| r.tax_rate_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListTaxRatesResponse {
            tax_rates: tax_rates.iter().map(Self::tax_rate_to_proto).collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "UpdateTaxRate")
    )]
    async fn update_tax_rate(
        &self,
        request: Request<UpdateTaxRateRequest>,
    ) -> Result<Response<UpdateTaxRateResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["UpdateTaxRate"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateTaxRate", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let tax_rate_id = Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateTaxRate", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tax_rate_id format")
        })?;

        let rate = if req.rate.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&req.rate).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateTaxRate", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid rate format")
            })?)
        };

        let effective_from = if req.effective_from.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_from, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["UpdateTaxRate", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid effective_from format")
                })?,
            )
        };

        let effective_to = if req.effective_to.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_to, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["UpdateTaxRate", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid effective_to format")
                })?,
            )
        };

        let calculation = if req.calculation == 0 {
            None
        } else {
            Some(match req.calculation {
                x if x == TaxCalculation::Inclusive as i32 => "inclusive".to_string(),
                _ => "exclusive".to_string(),
            })
        };

        let input = UpdateTaxRate {
            name: if req.name.is_empty() {
                None
            } else {
                Some(req.name)
            },
            rate,
            calculation,
            effective_from,
            effective_to,
            active: Some(req.active),
        };

        let tax_rate = self
            .db
            .update_tax_rate(tenant_id, tax_rate_id, &input)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to update tax rate");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateTaxRate", "error"])
                    .inc();
                Status::internal("Failed to update tax rate")
            })?;

        timer.observe_duration();

        match tax_rate {
            Some(rate) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateTaxRate", "ok"])
                    .inc();
                Ok(Response::new(UpdateTaxRateResponse {
                    tax_rate: Some(Self::tax_rate_to_proto(&rate)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateTaxRate", "not_found"])
                    .inc();
                Err(Status::not_found("Tax rate not found"))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Invoice Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "CreateInvoice",
            tenant_id,
            customer_id,
            invoice_id
        )
    )]
    async fn create_invoice(
        &self,
        request: Request<CreateInvoiceRequest>,
    ) -> Result<Response<CreateInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["CreateInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["CreateInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid customer_id format")
        })?;
        Span::current().record("customer_id", customer_id.to_string());

        let invoice_type = match req.invoice_type {
            x if x == ProtoInvoiceType::CreditNote as i32 => "credit_note",
            x if x == ProtoInvoiceType::Proforma as i32 => "proforma",
            _ => "standard",
        };

        let due_date = if req.due_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.due_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["CreateInvoice", "invalid_argument"])
                        .inc();
                    ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                    Status::invalid_argument("Invalid due_date format")
                })?,
            )
        };

        let reference_invoice_id = if req.reference_invoice_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.reference_invoice_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["CreateInvoice", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid reference_invoice_id format")
            })?)
        };

        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["CreateInvoice", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid metadata JSON")
            })?)
        };

        let address = req.billing_address.as_ref();

        let input = CreateInvoice {
            tenant_id,
            invoice_type: invoice_type.to_string(),
            customer_id,
            customer_name: req.customer_name,
            billing_line1: address.map(|a| a.line1.clone()).filter(|s| !s.is_empty()),
            billing_line2: address.map(|a| a.line2.clone()).filter(|s| !s.is_empty()),
            billing_city: address.map(|a| a.city.clone()).filter(|s| !s.is_empty()),
            billing_state: address.map(|a| a.state.clone()).filter(|s| !s.is_empty()),
            billing_postal_code: address
                .map(|a| a.postal_code.clone())
                .filter(|s| !s.is_empty()),
            billing_country: address.map(|a| a.country.clone()).filter(|s| !s.is_empty()),
            currency: req.currency,
            due_date,
            notes: if req.notes.is_empty() {
                None
            } else {
                Some(req.notes)
            },
            reference_invoice_id,
            metadata,
        };

        let invoice = self.db.create_invoice(&input).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to create invoice");
            GRPC_REQUESTS_TOTAL.with_label_values(&["CreateInvoice", "error"]).inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to create invoice")
        })?;

        Span::current().record("invoice_id", invoice.invoice_id.to_string());
        GRPC_REQUESTS_TOTAL
            .with_label_values(&["CreateInvoice", "ok"])
            .inc();
        INVOICES_TOTAL.with_label_values(&["draft"]).inc();
        timer.observe_duration();

        info!(tenant_id = %tenant_id, customer_id = %customer_id, invoice_id = %invoice.invoice_id, "Draft invoice created");

        Ok(Response::new(CreateInvoiceResponse {
            invoice: Some(Self::invoice_to_proto(&invoice, &[])),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetInvoice")
    )]
    async fn get_invoice(
        &self,
        request: Request<GetInvoiceRequest>,
    ) -> Result<Response<GetInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetInvoice", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetInvoice", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetInvoice", "error"])
                    .inc();
                Status::internal("Failed to get invoice")
            })?;

        timer.observe_duration();

        match invoice {
            Some(inv) => {
                let line_items = self
                    .db
                    .get_line_items(tenant_id, invoice_id)
                    .await
                    .map_err(|e| {
                        warn!(error = %e, "Failed to get line items");
                        Status::internal("Failed to get line items")
                    })?;
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetInvoice", "ok"])
                    .inc();
                Ok(Response::new(GetInvoiceResponse {
                    invoice: Some(Self::invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetInvoice", "not_found"])
                    .inc();
                Err(Status::not_found("Invoice not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "ListInvoices")
    )]
    async fn list_invoices(
        &self,
        request: Request<ListInvoicesRequest>,
    ) -> Result<Response<ListInvoicesResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["ListInvoices"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["ListInvoices", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let status = if req.status == ProtoInvoiceStatus::Unspecified as i32 {
            None
        } else {
            Some(match req.status {
                x if x == ProtoInvoiceStatus::Issued as i32 => InvoiceStatus::Issued,
                x if x == ProtoInvoiceStatus::Paid as i32 => InvoiceStatus::Paid,
                x if x == ProtoInvoiceStatus::Void as i32 => InvoiceStatus::Void,
                x if x == ProtoInvoiceStatus::Overdue as i32 => InvoiceStatus::Overdue,
                _ => InvoiceStatus::Draft,
            })
        };

        let customer_id = if req.customer_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.customer_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListInvoices", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid customer_id format")
            })?)
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListInvoices", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid start_date format")
                })?,
            )
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListInvoices", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid end_date format")
                })?,
            )
        };

        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListInvoices", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid page_token format")
            })?)
        };

        let page_size = if req.page_size <= 0 {
            20
        } else {
            req.page_size
        };

        let filter = ListInvoicesFilter {
            status,
            customer_id,
            start_date,
            end_date,
            page_size,
            page_token,
        };

        let invoices = self
            .db
            .list_invoices(tenant_id, &filter)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to list invoices");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListInvoices", "error"])
                    .inc();
                Status::internal("Failed to list invoices")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["ListInvoices", "ok"])
            .inc();
        timer.observe_duration();

        let next_page_token = if invoices.len() == filter.page_size as usize {
            invoices.last().map(|i| i.invoice_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListInvoicesResponse {
            invoices: invoices
                .iter()
                .map(|i| Self::invoice_to_proto(i, &[]))
                .collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "IssueInvoice",
            tenant_id,
            invoice_id
        )
    )]
    async fn issue_invoice(
        &self,
        request: Request<IssueInvoiceRequest>,
    ) -> Result<Response<IssueInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["IssueInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["IssueInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["IssueInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        let issue_date = if req.issue_date.is_empty() {
            chrono::Utc::now().date_naive()
        } else {
            NaiveDate::parse_from_str(&req.issue_date, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["IssueInvoice", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
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
                ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
                Status::internal("Failed to get invoice")
            })?;
        let existing_invoice = existing_invoice.ok_or_else(|| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["IssueInvoice", "not_found"])
                .inc();
            Status::not_found("Invoice not found")
        })?;

        let line_items = self
            .db
            .get_line_items(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get line items for ledger entry");
                ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
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
            GRPC_REQUESTS_TOTAL.with_label_values(&["IssueInvoice", "error"]).inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to issue invoice"),
            }
        })?;

        timer.observe_duration();

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
                    Status::internal("Failed to get line items")
                })?;
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["IssueInvoice", "ok"])
                    .inc();
                INVOICES_TOTAL.with_label_values(&["issued"]).inc();
                // Track invoice amount by currency for business metrics
                if let Some(amount) = inv.total.to_f64() {
                    INVOICE_AMOUNT_TOTAL
                        .with_label_values(&[&inv.currency])
                        .inc_by(amount);
                }
                info!(
                    tenant_id = %tenant_id,
                    invoice_id = %inv.invoice_id,
                    invoice_number = %inv.invoice_number.as_deref().unwrap_or(""),
                    total = %inv.total,
                    currency = %inv.currency,
                    "Invoice issued"
                );
                Ok(Response::new(IssueInvoiceResponse {
                    invoice: Some(Self::invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["IssueInvoice", "not_found"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
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
    async fn void_invoice(
        &self,
        request: Request<VoidInvoiceRequest>,
    ) -> Result<Response<VoidInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["VoidInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["VoidInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["VoidInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
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
                ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
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
                        ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
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
            GRPC_REQUESTS_TOTAL.with_label_values(&["VoidInvoice", "error"]).inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to void invoice"),
            }
        })?;

        timer.observe_duration();

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
                    Status::internal("Failed to get line items")
                })?;
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["VoidInvoice", "ok"])
                    .inc();
                INVOICES_TOTAL.with_label_values(&["void"]).inc();
                info!(tenant_id = %tenant_id, invoice_id = %inv.invoice_id, "Invoice voided");
                Ok(Response::new(VoidInvoiceResponse {
                    invoice: Some(Self::invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["VoidInvoice", "not_found"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
                Err(Status::not_found("Invoice not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "DeleteInvoice")
    )]
    async fn delete_invoice(
        &self,
        request: Request<DeleteInvoiceRequest>,
    ) -> Result<Response<DeleteInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["DeleteInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["DeleteInvoice", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["DeleteInvoice", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let deleted = self
            .db
            .delete_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to delete invoice");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["DeleteInvoice", "error"])
                    .inc();
                Status::internal("Failed to delete invoice")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["DeleteInvoice", "ok"])
            .inc();
        timer.observe_duration();

        Ok(Response::new(DeleteInvoiceResponse { success: deleted }))
    }

    // -------------------------------------------------------------------------
    // Line Item Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "AddLineItem")
    )]
    async fn add_line_item(
        &self,
        request: Request<AddLineItemRequest>,
    ) -> Result<Response<AddLineItemResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["AddLineItem"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["AddLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["AddLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let quantity = Decimal::from_str(&req.quantity).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["AddLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid quantity format")
        })?;

        let unit_price = Decimal::from_str(&req.unit_price).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["AddLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid unit_price format")
        })?;

        let tax_rate_id = if req.tax_rate_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["AddLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid tax_rate_id format")
            })?)
        };

        let ledger_account_id = if req.ledger_account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.ledger_account_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["AddLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid ledger_account_id format")
            })?)
        };

        let input = CreateLineItem {
            tenant_id,
            invoice_id,
            description: req.description,
            quantity,
            unit_price,
            tax_rate_id,
            ledger_account_id,
            sort_order: req.sort_order,
        };

        let line_item = self.db.add_line_item(&input).await.map_err(|e| {
            warn!(error = %e, "Failed to add line item");
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["AddLineItem", "error"])
                .inc();
            match e {
                service_core::error::AppError::BadRequest(err) => {
                    Status::failed_precondition(err.to_string())
                }
                service_core::error::AppError::NotFound(err) => Status::not_found(err.to_string()),
                _ => Status::internal("Failed to add line item"),
            }
        })?;

        // Get updated invoice
        let invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice");
                Status::internal("Failed to get invoice")
            })?
            .ok_or_else(|| Status::not_found("Invoice not found"))?;

        let line_items = self
            .db
            .get_line_items(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get line items");
                Status::internal("Failed to get line items")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["AddLineItem", "ok"])
            .inc();
        timer.observe_duration();

        Ok(Response::new(AddLineItemResponse {
            line_item: Some(Self::line_item_to_proto(&line_item)),
            invoice: Some(Self::invoice_to_proto(&invoice, &line_items)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "UpdateLineItem")
    )]
    async fn update_line_item(
        &self,
        request: Request<UpdateLineItemRequest>,
    ) -> Result<Response<UpdateLineItemResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["UpdateLineItem"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let line_item_id = Uuid::parse_str(&req.line_item_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid line_item_id format")
        })?;

        let quantity = if req.quantity.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&req.quantity).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid quantity format")
            })?)
        };

        let unit_price = if req.unit_price.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&req.unit_price).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid unit_price format")
            })?)
        };

        let tax_rate_id = if req.tax_rate_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid tax_rate_id format")
            })?)
        };

        let ledger_account_id = if req.ledger_account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.ledger_account_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid ledger_account_id format")
            })?)
        };

        let input = UpdateLineItem {
            description: if req.description.is_empty() {
                None
            } else {
                Some(req.description)
            },
            quantity,
            unit_price,
            tax_rate_id,
            ledger_account_id,
            sort_order: if req.sort_order == 0 {
                None
            } else {
                Some(req.sort_order)
            },
        };

        let line_item = self
            .db
            .update_line_item(tenant_id, invoice_id, line_item_id, &input)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to update line item");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "error"])
                    .inc();
                match e {
                    service_core::error::AppError::BadRequest(err) => {
                        Status::failed_precondition(err.to_string())
                    }
                    _ => Status::internal("Failed to update line item"),
                }
            })?;

        timer.observe_duration();

        match line_item {
            Some(item) => {
                let invoice = self
                    .db
                    .get_invoice(tenant_id, invoice_id)
                    .await
                    .map_err(|e| {
                        warn!(error = %e, "Failed to get invoice");
                        Status::internal("Failed to get invoice")
                    })?
                    .ok_or_else(|| Status::not_found("Invoice not found"))?;

                let line_items = self
                    .db
                    .get_line_items(tenant_id, invoice_id)
                    .await
                    .map_err(|e| {
                        warn!(error = %e, "Failed to get line items");
                        Status::internal("Failed to get line items")
                    })?;

                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "ok"])
                    .inc();
                Ok(Response::new(UpdateLineItemResponse {
                    line_item: Some(Self::line_item_to_proto(&item)),
                    invoice: Some(Self::invoice_to_proto(&invoice, &line_items)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateLineItem", "not_found"])
                    .inc();
                Err(Status::not_found("Line item not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "RemoveLineItem")
    )]
    async fn remove_line_item(
        &self,
        request: Request<RemoveLineItemRequest>,
    ) -> Result<Response<RemoveLineItemResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["RemoveLineItem"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RemoveLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RemoveLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let line_item_id = Uuid::parse_str(&req.line_item_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RemoveLineItem", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid line_item_id format")
        })?;

        let removed = self
            .db
            .remove_line_item(tenant_id, invoice_id, line_item_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to remove line item");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["RemoveLineItem", "error"])
                    .inc();
                match e {
                    service_core::error::AppError::BadRequest(err) => {
                        Status::failed_precondition(err.to_string())
                    }
                    _ => Status::internal("Failed to remove line item"),
                }
            })?;

        if !removed {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RemoveLineItem", "not_found"])
                .inc();
            return Err(Status::not_found("Line item not found"));
        }

        // Get updated invoice
        let invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice");
                Status::internal("Failed to get invoice")
            })?
            .ok_or_else(|| Status::not_found("Invoice not found"))?;

        let line_items = self
            .db
            .get_line_items(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get line items");
                Status::internal("Failed to get line items")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["RemoveLineItem", "ok"])
            .inc();
        timer.observe_duration();

        Ok(Response::new(RemoveLineItemResponse {
            invoice: Some(Self::invoice_to_proto(&invoice, &line_items)),
        }))
    }

    // -------------------------------------------------------------------------
    // Receipt Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "RecordPayment",
            tenant_id,
            invoice_id,
            receipt_id
        )
    )]
    async fn record_payment(
        &self,
        request: Request<RecordPaymentRequest>,
    ) -> Result<Response<RecordPaymentResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["RecordPayment"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RecordPayment", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RecordPayment", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        let amount = Decimal::from_str(&req.amount).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RecordPayment", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid amount format")
        })?;

        let payment_date =
            NaiveDate::parse_from_str(&req.payment_date, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["RecordPayment", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid payment_date format")
            })?;

        // Get invoice for ledger entry
        let existing_invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice for payment ledger entry");
                ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
                Status::internal("Failed to get invoice")
            })?;
        let existing_invoice = existing_invoice.ok_or_else(|| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["RecordPayment", "not_found"])
                .inc();
            Status::not_found("Invoice not found")
        })?;

        // Create ledger entry for payment: Debit Cash, Credit A/R
        let journal_id = if let Some(ref ledger_client) = self.ledger_client {
            // Convention: Cash account based on payment method, A/R from invoice
            let cash_account = format!(
                "CASH-{}-{}",
                req.payment_method.to_uppercase(),
                existing_invoice.currency
            );
            let ar_account_id = format!("AR-{}", existing_invoice.currency);
            let amount_str = format_decimal(&amount);
            // Use invoice_id + payment_date + amount for idempotency
            let idempotency_key = format!("payment-{}-{}-{}", invoice_id, payment_date, amount_str);

            let entries = vec![
                TransactionEntry::debit(&cash_account, &amount_str),
                TransactionEntry::credit(&ar_account_id, &amount_str),
            ];

            let metadata = serde_json::json!({
                "source": "invoicing-service",
                "invoice_id": invoice_id.to_string(),
                "customer_id": existing_invoice.customer_id.to_string(),
                "payment_method": &req.payment_method,
                "amount": &amount_str,
            })
            .to_string();

            match ledger_client
                .post_transaction(
                    &tenant_id.to_string(),
                    entries,
                    Some(&payment_date.to_string()),
                    &idempotency_key,
                    Some(&metadata),
                )
                .await
            {
                Ok(response) => {
                    if let Some(ref txn) = response.transaction {
                        info!(journal_id = %txn.journal_id, "Ledger entry created for payment");
                        Uuid::parse_str(&txn.journal_id).ok()
                    } else {
                        warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, "Ledger response missing transaction");
                        None
                    }
                }
                Err(e) => {
                    // Log but don't fail - ledger integration is optional enhancement
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to create ledger entry for payment");
                    None
                }
            }
        } else {
            None
        };

        let input = CreateReceipt {
            tenant_id,
            invoice_id,
            amount,
            payment_method: req.payment_method.clone(),
            payment_reference: if req.payment_reference.is_empty() {
                None
            } else {
                Some(req.payment_reference)
            },
            payment_date,
            journal_id,
            notes: if req.notes.is_empty() {
                None
            } else {
                Some(req.notes)
            },
        };

        let receipt = self.db.record_payment(&input).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to record payment");
            GRPC_REQUESTS_TOTAL.with_label_values(&["RecordPayment", "error"]).inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                service_core::error::AppError::NotFound(err) => {
                    ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
                    Status::not_found(err.to_string())
                }
                _ => Status::internal("Failed to record payment"),
            }
        })?;

        Span::current().record("receipt_id", receipt.receipt_id.to_string());

        // Get updated invoice
        let invoice = self.db.get_invoice(tenant_id, invoice_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get invoice");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to get invoice")
        })?.ok_or_else(|| Status::not_found("Invoice not found"))?;

        let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to get line items")
        })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["RecordPayment", "ok"])
            .inc();
        RECEIPTS_TOTAL
            .with_label_values(&[&req.payment_method])
            .inc();
        // Track payment amount by currency for business metrics
        if let Some(payment_amount) = receipt.amount.to_f64() {
            PAYMENT_AMOUNT_TOTAL
                .with_label_values(&[&receipt.currency])
                .inc_by(payment_amount);
        }
        timer.observe_duration();

        info!(
            tenant_id = %tenant_id,
            invoice_id = %invoice_id,
            receipt_id = %receipt.receipt_id,
            receipt_number = %receipt.receipt_number,
            amount = %receipt.amount,
            currency = %receipt.currency,
            payment_method = %receipt.payment_method,
            "Payment recorded"
        );

        Ok(Response::new(RecordPaymentResponse {
            receipt: Some(Self::receipt_to_proto(&receipt)),
            invoice: Some(Self::invoice_to_proto(&invoice, &line_items)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetReceipt")
    )]
    async fn get_receipt(
        &self,
        request: Request<GetReceiptRequest>,
    ) -> Result<Response<GetReceiptResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GetReceipt"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetReceipt", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let receipt_id = Uuid::parse_str(&req.receipt_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GetReceipt", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid receipt_id format")
        })?;

        let receipt = self
            .db
            .get_receipt(tenant_id, receipt_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get receipt");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetReceipt", "error"])
                    .inc();
                Status::internal("Failed to get receipt")
            })?;

        timer.observe_duration();

        match receipt {
            Some(r) => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetReceipt", "ok"])
                    .inc();
                Ok(Response::new(GetReceiptResponse {
                    receipt: Some(Self::receipt_to_proto(&r)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GetReceipt", "not_found"])
                    .inc();
                Err(Status::not_found("Receipt not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "ListReceipts")
    )]
    async fn list_receipts(
        &self,
        request: Request<ListReceiptsRequest>,
    ) -> Result<Response<ListReceiptsResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["ListReceipts"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["ListReceipts", "invalid_argument"])
                .inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = if req.invoice_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.invoice_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListReceipts", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid invoice_id format")
            })?)
        };

        let customer_id = if req.customer_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.customer_id).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListReceipts", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid customer_id format")
            })?)
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListReceipts", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid start_date format")
                })?,
            )
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["ListReceipts", "invalid_argument"])
                        .inc();
                    Status::invalid_argument("Invalid end_date format")
                })?,
            )
        };

        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListReceipts", "invalid_argument"])
                    .inc();
                Status::invalid_argument("Invalid page_token format")
            })?)
        };

        let page_size = if req.page_size <= 0 {
            20
        } else {
            req.page_size
        };

        let filter = ListReceiptsFilter {
            invoice_id,
            customer_id,
            start_date,
            end_date,
            page_size,
            page_token,
        };

        let receipts = self
            .db
            .list_receipts(tenant_id, &filter)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to list receipts");
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["ListReceipts", "error"])
                    .inc();
                Status::internal("Failed to list receipts")
            })?;

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["ListReceipts", "ok"])
            .inc();
        timer.observe_duration();

        let next_page_token = if receipts.len() == filter.page_size as usize {
            receipts.last().map(|r| r.receipt_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListReceiptsResponse {
            receipts: receipts.iter().map(Self::receipt_to_proto).collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    // -------------------------------------------------------------------------
    // Statement Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateStatement",
            tenant_id,
            customer_id
        )
    )]
    async fn generate_statement(
        &self,
        request: Request<GenerateStatementRequest>,
    ) -> Result<Response<GenerateStatementResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GenerateStatement"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatement", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatement", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid customer_id format")
        })?;
        Span::current().record("customer_id", customer_id.to_string());

        let period_start =
            NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GenerateStatement", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid period_start format")
            })?;

        let period_end = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d").map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatement", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid period_end format")
        })?;

        if period_start > period_end {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatement", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            return Err(Status::invalid_argument(
                "period_start must be before period_end",
            ));
        }

        // Get customer info from most recent invoice
        let customer_info = self.db.get_customer_info(tenant_id, customer_id).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get customer info");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
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
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GenerateStatement", "not_found"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
                return Err(Status::not_found("No invoices found for customer"));
            }
        };

        // Calculate opening balance
        let opening_balance = self.db.calculate_opening_balance(tenant_id, customer_id, period_start).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to calculate opening balance");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to calculate opening balance")
        })?;

        // Get invoices in period
        let invoices = self.db.get_invoices_for_statement(tenant_id, customer_id, period_start, period_end).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get invoices for statement");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            Status::internal("Failed to get invoices")
        })?;

        // Get receipts in period
        let receipts = self.db.get_receipts_for_statement(tenant_id, customer_id, period_start, period_end).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, customer_id = %customer_id, error = %e, "Failed to get receipts for statement");
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
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

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GenerateStatement", "ok"])
            .inc();
        timer.observe_duration();

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
                generated_at: Some(Self::datetime_to_timestamp(chrono::Utc::now())),
            }),
        }))
    }

    // -------------------------------------------------------------------------
    // Update Invoice (draft only)
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "UpdateInvoice",
            tenant_id,
            invoice_id
        )
    )]
    async fn update_invoice(
        &self,
        request: Request<UpdateInvoiceRequest>,
    ) -> Result<Response<UpdateInvoiceResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["UpdateInvoice"])
            .start_timer();
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["UpdateInvoice", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        let due_date = if req.due_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.due_date, "%Y-%m-%d").map_err(|_| {
                    GRPC_REQUESTS_TOTAL
                        .with_label_values(&["UpdateInvoice", "invalid_argument"])
                        .inc();
                    ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                    Status::invalid_argument("Invalid due_date format")
                })?,
            )
        };

        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateInvoice", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid metadata JSON")
            })?)
        };

        let address = req.billing_address.as_ref();

        let input = UpdateInvoice {
            customer_name: if req.customer_name.is_empty() {
                None
            } else {
                Some(req.customer_name)
            },
            billing_line1: address.and_then(|a| {
                if a.line1.is_empty() {
                    None
                } else {
                    Some(a.line1.clone())
                }
            }),
            billing_line2: address.and_then(|a| {
                if a.line2.is_empty() {
                    None
                } else {
                    Some(a.line2.clone())
                }
            }),
            billing_city: address.and_then(|a| {
                if a.city.is_empty() {
                    None
                } else {
                    Some(a.city.clone())
                }
            }),
            billing_state: address.and_then(|a| {
                if a.state.is_empty() {
                    None
                } else {
                    Some(a.state.clone())
                }
            }),
            billing_postal_code: address.and_then(|a| {
                if a.postal_code.is_empty() {
                    None
                } else {
                    Some(a.postal_code.clone())
                }
            }),
            billing_country: address.and_then(|a| {
                if a.country.is_empty() {
                    None
                } else {
                    Some(a.country.clone())
                }
            }),
            due_date,
            notes: if req.notes.is_empty() {
                None
            } else {
                Some(req.notes)
            },
            metadata,
        };

        let invoice = self.db.update_invoice(tenant_id, invoice_id, &input).await.map_err(|e| {
            warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to update invoice");
            GRPC_REQUESTS_TOTAL.with_label_values(&["UpdateInvoice", "error"]).inc();
            ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to update invoice"),
            }
        })?;

        timer.observe_duration();

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    ERRORS_TOTAL.with_label_values(&["db_error"]).inc();
                    Status::internal("Failed to get line items")
                })?;
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateInvoice", "ok"])
                    .inc();
                info!(tenant_id = %tenant_id, invoice_id = %inv.invoice_id, "Invoice updated");
                Ok(Response::new(UpdateInvoiceResponse {
                    invoice: Some(Self::invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["UpdateInvoice", "not_found"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["not_found"]).inc();
                Err(Status::not_found("Invoice not found"))
            }
        }
    }

    // -------------------------------------------------------------------------
    // PDF Generation Methods
    // -------------------------------------------------------------------------

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateInvoicePdf",
            tenant_id,
            invoice_id
        )
    )]
    async fn generate_invoice_pdf(
        &self,
        request: Request<GenerateInvoicePdfRequest>,
    ) -> Result<Response<GenerateInvoicePdfResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GenerateInvoicePdf"])
            .start_timer();
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateInvoicePdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateInvoicePdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", _invoice_id.to_string());

        // TODO: Implement PDF generation
        // 1. Fetch invoice with line items
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes and filename

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GenerateInvoicePdf", "unimplemented"])
            .inc();
        timer.observe_duration();

        Err(Status::unimplemented(
            "GenerateInvoicePdf not yet implemented",
        ))
    }

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateReceiptPdf",
            tenant_id,
            receipt_id
        )
    )]
    async fn generate_receipt_pdf(
        &self,
        request: Request<GenerateReceiptPdfRequest>,
    ) -> Result<Response<GenerateReceiptPdfResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GenerateReceiptPdf"])
            .start_timer();
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateReceiptPdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _receipt_id = Uuid::parse_str(&req.receipt_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateReceiptPdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid receipt_id format")
        })?;
        Span::current().record("receipt_id", _receipt_id.to_string());

        // TODO: Implement PDF generation
        // 1. Fetch receipt
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes and filename

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GenerateReceiptPdf", "unimplemented"])
            .inc();
        timer.observe_duration();

        Err(Status::unimplemented(
            "GenerateReceiptPdf not yet implemented",
        ))
    }

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateStatementPdf",
            tenant_id,
            customer_id
        )
    )]
    async fn generate_statement_pdf(
        &self,
        request: Request<GenerateStatementPdfRequest>,
    ) -> Result<Response<GenerateStatementPdfResponse>, Status> {
        let timer = GRPC_REQUEST_DURATION
            .with_label_values(&["GenerateStatementPdf"])
            .start_timer();
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatementPdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatementPdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid customer_id format")
        })?;
        Span::current().record("customer_id", _customer_id.to_string());

        let _period_start =
            NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d").map_err(|_| {
                GRPC_REQUESTS_TOTAL
                    .with_label_values(&["GenerateStatementPdf", "invalid_argument"])
                    .inc();
                ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
                Status::invalid_argument("Invalid period_start format")
            })?;

        let _period_end = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d").map_err(|_| {
            GRPC_REQUESTS_TOTAL
                .with_label_values(&["GenerateStatementPdf", "invalid_argument"])
                .inc();
            ERRORS_TOTAL.with_label_values(&["validation_error"]).inc();
            Status::invalid_argument("Invalid period_end format")
        })?;

        // TODO: Implement statement PDF generation
        // 1. Generate statement (same logic as GenerateStatement)
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes, filename, and statement data

        GRPC_REQUESTS_TOTAL
            .with_label_values(&["GenerateStatementPdf", "unimplemented"])
            .inc();
        timer.observe_duration();

        Err(Status::unimplemented(
            "GenerateStatementPdf not yet implemented",
        ))
    }
}
