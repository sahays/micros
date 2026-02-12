//! Invoice CRUD gRPC handler implementations.

use crate::grpc::helpers::invoice_to_proto;
use crate::grpc::proto::{
    CreateInvoiceRequest, CreateInvoiceResponse, DeleteInvoiceRequest, DeleteInvoiceResponse,
    GetInvoiceRequest, GetInvoiceResponse, InvoiceStatus as ProtoInvoiceStatus,
    InvoiceType as ProtoInvoiceType, ListInvoicesRequest, ListInvoicesResponse,
    UpdateInvoiceRequest, UpdateInvoiceResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use crate::models::{CreateInvoice, InvoiceStatus, ListInvoicesFilter, UpdateInvoice};
use chrono::NaiveDate;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
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
    pub(crate) async fn handle_create_invoice(
        &self,
        request: Request<CreateInvoiceRequest>,
    ) -> Result<Response<CreateInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let customer_id = Uuid::parse_str(&req.customer_id).map_err(|_| {
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
                    Status::invalid_argument("Invalid due_date format")
                })?,
            )
        };

        let reference_invoice_id = if req.reference_invoice_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.reference_invoice_id).map_err(|_| {
                Status::invalid_argument("Invalid reference_invoice_id format")
            })?)
        };

        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
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
            Status::internal("Failed to create invoice")
        })?;

        Span::current().record("invoice_id", invoice.invoice_id.to_string());

        info!(tenant_id = %tenant_id, customer_id = %customer_id, invoice_id = %invoice.invoice_id, "Draft invoice created");

        Ok(Response::new(CreateInvoiceResponse {
            invoice: Some(invoice_to_proto(&invoice, &[])),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetInvoice")
    )]
    pub(crate) async fn handle_get_invoice(
        &self,
        request: Request<GetInvoiceRequest>,
    ) -> Result<Response<GetInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let invoice = self
            .db
            .get_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get invoice");
                Status::internal("Failed to get invoice")
            })?;

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
                Ok(Response::new(GetInvoiceResponse {
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
        fields(service = "invoicing-service", method = "ListInvoices")
    )]
    pub(crate) async fn handle_list_invoices(
        &self,
        request: Request<ListInvoicesRequest>,
    ) -> Result<Response<ListInvoicesResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
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
                Status::invalid_argument("Invalid customer_id format")
            })?)
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d").map_err(|_| {
                    Status::invalid_argument("Invalid start_date format")
                })?,
            )
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d").map_err(|_| {
                    Status::invalid_argument("Invalid end_date format")
                })?,
            )
        };

        let page_token = if req.page_token.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.page_token).map_err(|_| {
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
                Status::internal("Failed to list invoices")
            })?;

        let next_page_token = if invoices.len() == filter.page_size as usize {
            invoices.last().map(|i| i.invoice_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListInvoicesResponse {
            invoices: invoices.iter().map(|i| invoice_to_proto(i, &[])).collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "DeleteInvoice")
    )]
    pub(crate) async fn handle_delete_invoice(
        &self,
        request: Request<DeleteInvoiceRequest>,
    ) -> Result<Response<DeleteInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;

        let deleted = self
            .db
            .delete_invoice(tenant_id, invoice_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to delete invoice");
                Status::internal("Failed to delete invoice")
            })?;

        Ok(Response::new(DeleteInvoiceResponse { success: deleted }))
    }

    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "UpdateInvoice",
            tenant_id,
            invoice_id
        )
    )]
    pub(crate) async fn handle_update_invoice(
        &self,
        request: Request<UpdateInvoiceRequest>,
    ) -> Result<Response<UpdateInvoiceResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let invoice_id = Uuid::parse_str(&req.invoice_id).map_err(|_| {
            Status::invalid_argument("Invalid invoice_id format")
        })?;
        Span::current().record("invoice_id", invoice_id.to_string());

        let due_date = if req.due_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.due_date, "%Y-%m-%d").map_err(|_| {
                    Status::invalid_argument("Invalid due_date format")
                })?,
            )
        };

        let metadata = if req.metadata.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&req.metadata).map_err(|_| {
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
            match e {
                service_core::error::AppError::BadRequest(err) => Status::failed_precondition(err.to_string()),
                _ => Status::internal("Failed to update invoice"),
            }
        })?;

        match invoice {
            Some(inv) => {
                let line_items = self.db.get_line_items(tenant_id, invoice_id).await.map_err(|e| {
                    warn!(tenant_id = %tenant_id, invoice_id = %invoice_id, error = %e, "Failed to get line items");
                    Status::internal("Failed to get line items")
                })?;
                info!(tenant_id = %tenant_id, invoice_id = %inv.invoice_id, "Invoice updated");
                Ok(Response::new(UpdateInvoiceResponse {
                    invoice: Some(invoice_to_proto(&inv, &line_items)),
                }))
            }
            None => {
                Err(Status::not_found("Invoice not found"))
            }
        }
    }
}
