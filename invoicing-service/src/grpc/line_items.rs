//! Line item gRPC handler implementations.

use crate::grpc::helpers::{invoice_to_proto, line_item_to_proto};
use crate::grpc::proto::{
    AddLineItemRequest, AddLineItemResponse, RemoveLineItemRequest, RemoveLineItemResponse,
    UpdateLineItemRequest, UpdateLineItemResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use crate::models::{CreateLineItem, UpdateLineItem};
use crate::services::metrics::{GRPC_REQUESTS_TOTAL, GRPC_REQUEST_DURATION};
use rust_decimal::Decimal;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use tracing::{instrument, warn};
use uuid::Uuid;

impl InvoicingServiceImpl {
    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "AddLineItem")
    )]
    pub(crate) async fn handle_add_line_item(
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
            line_item: Some(line_item_to_proto(&line_item)),
            invoice: Some(invoice_to_proto(&invoice, &line_items)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "UpdateLineItem")
    )]
    pub(crate) async fn handle_update_line_item(
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
                    line_item: Some(line_item_to_proto(&item)),
                    invoice: Some(invoice_to_proto(&invoice, &line_items)),
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
    pub(crate) async fn handle_remove_line_item(
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
            invoice: Some(invoice_to_proto(&invoice, &line_items)),
        }))
    }
}
