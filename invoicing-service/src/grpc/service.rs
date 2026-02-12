//! InvoicingService gRPC implementation.

use crate::grpc::proto::{
    invoicing_service_server::InvoicingService, AddLineItemRequest, AddLineItemResponse,
    CreateInvoiceRequest, CreateInvoiceResponse, CreateTaxRateRequest, CreateTaxRateResponse,
    DeleteInvoiceRequest, DeleteInvoiceResponse, GenerateInvoicePdfRequest,
    GenerateInvoicePdfResponse, GenerateReceiptPdfRequest, GenerateReceiptPdfResponse,
    GenerateStatementPdfRequest, GenerateStatementPdfResponse, GenerateStatementRequest,
    GenerateStatementResponse, GetInvoiceRequest, GetInvoiceResponse, GetReceiptRequest,
    GetReceiptResponse, GetTaxRateRequest, GetTaxRateResponse, IssueInvoiceRequest,
    IssueInvoiceResponse, ListInvoicesRequest, ListInvoicesResponse, ListReceiptsRequest,
    ListReceiptsResponse, ListTaxRatesRequest, ListTaxRatesResponse, RecordPaymentRequest,
    RecordPaymentResponse, RemoveLineItemRequest, RemoveLineItemResponse, UpdateInvoiceRequest,
    UpdateInvoiceResponse, UpdateLineItemRequest, UpdateLineItemResponse, UpdateTaxRateRequest,
    UpdateTaxRateResponse, VoidInvoiceRequest, VoidInvoiceResponse,
};
use crate::services::Database;
use service_core::grpc::LedgerClient;
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// InvoicingService implementation.
pub struct InvoicingServiceImpl {
    pub(crate) db: Arc<Database>,
    pub(crate) ledger_client: Option<Arc<LedgerClient>>,
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
}

#[tonic::async_trait]
impl InvoicingService for InvoicingServiceImpl {
    // Tax Rate Methods
    async fn create_tax_rate(
        &self,
        request: Request<CreateTaxRateRequest>,
    ) -> Result<Response<CreateTaxRateResponse>, Status> {
        self.handle_create_tax_rate(request).await
    }

    async fn get_tax_rate(
        &self,
        request: Request<GetTaxRateRequest>,
    ) -> Result<Response<GetTaxRateResponse>, Status> {
        self.handle_get_tax_rate(request).await
    }

    async fn list_tax_rates(
        &self,
        request: Request<ListTaxRatesRequest>,
    ) -> Result<Response<ListTaxRatesResponse>, Status> {
        self.handle_list_tax_rates(request).await
    }

    async fn update_tax_rate(
        &self,
        request: Request<UpdateTaxRateRequest>,
    ) -> Result<Response<UpdateTaxRateResponse>, Status> {
        self.handle_update_tax_rate(request).await
    }

    // Invoice Methods
    async fn create_invoice(
        &self,
        request: Request<CreateInvoiceRequest>,
    ) -> Result<Response<CreateInvoiceResponse>, Status> {
        self.handle_create_invoice(request).await
    }

    async fn get_invoice(
        &self,
        request: Request<GetInvoiceRequest>,
    ) -> Result<Response<GetInvoiceResponse>, Status> {
        self.handle_get_invoice(request).await
    }

    async fn list_invoices(
        &self,
        request: Request<ListInvoicesRequest>,
    ) -> Result<Response<ListInvoicesResponse>, Status> {
        self.handle_list_invoices(request).await
    }

    async fn issue_invoice(
        &self,
        request: Request<IssueInvoiceRequest>,
    ) -> Result<Response<IssueInvoiceResponse>, Status> {
        self.handle_issue_invoice(request).await
    }

    async fn void_invoice(
        &self,
        request: Request<VoidInvoiceRequest>,
    ) -> Result<Response<VoidInvoiceResponse>, Status> {
        self.handle_void_invoice(request).await
    }

    async fn delete_invoice(
        &self,
        request: Request<DeleteInvoiceRequest>,
    ) -> Result<Response<DeleteInvoiceResponse>, Status> {
        self.handle_delete_invoice(request).await
    }

    async fn update_invoice(
        &self,
        request: Request<UpdateInvoiceRequest>,
    ) -> Result<Response<UpdateInvoiceResponse>, Status> {
        self.handle_update_invoice(request).await
    }

    // Line Item Methods
    async fn add_line_item(
        &self,
        request: Request<AddLineItemRequest>,
    ) -> Result<Response<AddLineItemResponse>, Status> {
        self.handle_add_line_item(request).await
    }

    async fn update_line_item(
        &self,
        request: Request<UpdateLineItemRequest>,
    ) -> Result<Response<UpdateLineItemResponse>, Status> {
        self.handle_update_line_item(request).await
    }

    async fn remove_line_item(
        &self,
        request: Request<RemoveLineItemRequest>,
    ) -> Result<Response<RemoveLineItemResponse>, Status> {
        self.handle_remove_line_item(request).await
    }

    // Receipt Methods
    async fn record_payment(
        &self,
        request: Request<RecordPaymentRequest>,
    ) -> Result<Response<RecordPaymentResponse>, Status> {
        self.handle_record_payment(request).await
    }

    async fn get_receipt(
        &self,
        request: Request<GetReceiptRequest>,
    ) -> Result<Response<GetReceiptResponse>, Status> {
        self.handle_get_receipt(request).await
    }

    async fn list_receipts(
        &self,
        request: Request<ListReceiptsRequest>,
    ) -> Result<Response<ListReceiptsResponse>, Status> {
        self.handle_list_receipts(request).await
    }

    // Statement Methods
    async fn generate_statement(
        &self,
        request: Request<GenerateStatementRequest>,
    ) -> Result<Response<GenerateStatementResponse>, Status> {
        self.handle_generate_statement(request).await
    }

    // PDF Methods
    async fn generate_invoice_pdf(
        &self,
        request: Request<GenerateInvoicePdfRequest>,
    ) -> Result<Response<GenerateInvoicePdfResponse>, Status> {
        self.handle_generate_invoice_pdf(request).await
    }

    async fn generate_receipt_pdf(
        &self,
        request: Request<GenerateReceiptPdfRequest>,
    ) -> Result<Response<GenerateReceiptPdfResponse>, Status> {
        self.handle_generate_receipt_pdf(request).await
    }

    async fn generate_statement_pdf(
        &self,
        request: Request<GenerateStatementPdfRequest>,
    ) -> Result<Response<GenerateStatementPdfResponse>, Status> {
        self.handle_generate_statement_pdf(request).await
    }
}
