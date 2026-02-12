//! PDF generation gRPC handler implementations.

use crate::grpc::proto::{
    GenerateInvoicePdfRequest, GenerateInvoicePdfResponse, GenerateReceiptPdfRequest,
    GenerateReceiptPdfResponse, GenerateStatementPdfRequest, GenerateStatementPdfResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use chrono::NaiveDate;
use tonic::{Request, Response, Status};
use tracing::{instrument, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "GenerateInvoicePdf",
            tenant_id,
            invoice_id
        )
    )]
    pub(crate) async fn handle_generate_invoice_pdf(
        &self,
        request: Request<GenerateInvoicePdfRequest>,
    ) -> Result<Response<GenerateInvoicePdfResponse>, Status> {
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", _invoice_id.to_string());

        // TODO: Implement PDF generation
        // 1. Fetch invoice with line items
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes and filename

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
    pub(crate) async fn handle_generate_receipt_pdf(
        &self,
        request: Request<GenerateReceiptPdfRequest>,
    ) -> Result<Response<GenerateReceiptPdfResponse>, Status> {
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _receipt_id = Uuid::parse_str(&req.receipt_id).map_err(|_| {
            Status::invalid_argument("Invalid receipt_id format")
        })?;
        Span::current().record("receipt_id", _receipt_id.to_string());

        // TODO: Implement PDF generation
        // 1. Fetch receipt
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes and filename

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
    pub(crate) async fn handle_generate_statement_pdf(
        &self,
        request: Request<GenerateStatementPdfRequest>,
    ) -> Result<Response<GenerateStatementPdfResponse>, Status> {
        let req = request.into_inner();

        let _tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", _tenant_id.to_string());

        let _customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
            Status::invalid_argument("Invalid customer_id format")
        })?;
        Span::current().record("customer_id", _customer_id.to_string());

        let _period_start =
            NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid period_start format")
            })?;

        let _period_end = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d").map_err(|_| {
            Status::invalid_argument("Invalid period_end format")
        })?;

        // TODO: Implement statement PDF generation
        // 1. Generate statement (same logic as GenerateStatement)
        // 2. Generate PDF using printpdf or typst
        // 3. Return PDF bytes, filename, and statement data

        Err(Status::unimplemented(
            "GenerateStatementPdf not yet implemented",
        ))
    }
}
