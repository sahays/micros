//! Receipt gRPC handler implementations.

use crate::grpc::helpers::{format_decimal, invoice_to_proto, receipt_to_proto};
use crate::grpc::proto::{
    GetReceiptRequest, GetReceiptResponse, ListReceiptsRequest, ListReceiptsResponse,
    RecordPaymentRequest, RecordPaymentResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use crate::models::{CreateReceipt, ListReceiptsFilter};
use crate::services::metrics::{
    ERRORS_TOTAL, GRPC_REQUESTS_TOTAL, GRPC_REQUEST_DURATION, PAYMENT_AMOUNT_TOTAL, RECEIPTS_TOTAL,
};
use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use service_core::grpc::TransactionEntry;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
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
    pub(crate) async fn handle_record_payment(
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
            receipt: Some(receipt_to_proto(&receipt)),
            invoice: Some(invoice_to_proto(&invoice, &line_items)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetReceipt")
    )]
    pub(crate) async fn handle_get_receipt(
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
                    receipt: Some(receipt_to_proto(&r)),
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
    pub(crate) async fn handle_list_receipts(
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
            receipts: receipts.iter().map(receipt_to_proto).collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }
}
