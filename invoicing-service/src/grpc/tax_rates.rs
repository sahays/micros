//! Tax rate gRPC handler implementations.

use crate::grpc::helpers::tax_rate_to_proto;
use crate::grpc::proto::{
    CreateTaxRateRequest, CreateTaxRateResponse, GetTaxRateRequest, GetTaxRateResponse,
    ListTaxRatesRequest, ListTaxRatesResponse, TaxCalculation, UpdateTaxRateRequest,
    UpdateTaxRateResponse,
};
use crate::grpc::service::InvoicingServiceImpl;
use crate::models::{CreateTaxRate, UpdateTaxRate};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn, Span};
use uuid::Uuid;

impl InvoicingServiceImpl {
    #[instrument(
        skip(self, request),
        fields(
            service = "invoicing-service",
            method = "CreateTaxRate",
            tenant_id,
            tax_rate_id
        )
    )]
    pub(crate) async fn handle_create_tax_rate(
        &self,
        request: Request<CreateTaxRateRequest>,
    ) -> Result<Response<CreateTaxRateResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;
        Span::current().record("tenant_id", tenant_id.to_string());

        let rate = Decimal::from_str(&req.rate).map_err(|_| {
            Status::invalid_argument("Invalid rate format")
        })?;

        let effective_from =
            NaiveDate::parse_from_str(&req.effective_from, "%Y-%m-%d").map_err(|_| {
                Status::invalid_argument("Invalid effective_from format")
            })?;

        let effective_to = if req.effective_to.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_to, "%Y-%m-%d").map_err(|_| {
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
            Status::internal("Failed to create tax rate")
        })?;

        Span::current().record("tax_rate_id", tax_rate.tax_rate_id.to_string());

        info!(tenant_id = %tenant_id, tax_rate_id = %tax_rate.tax_rate_id, "Tax rate created");

        Ok(Response::new(CreateTaxRateResponse {
            tax_rate: Some(tax_rate_to_proto(&tax_rate)),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "GetTaxRate")
    )]
    pub(crate) async fn handle_get_tax_rate(
        &self,
        request: Request<GetTaxRateRequest>,
    ) -> Result<Response<GetTaxRateResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let tax_rate_id = Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
            Status::invalid_argument("Invalid tax_rate_id format")
        })?;

        let tax_rate = self
            .db
            .get_tax_rate(tenant_id, tax_rate_id)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to get tax rate");
                Status::internal("Failed to get tax rate")
            })?;

        match tax_rate {
            Some(rate) => {
                Ok(Response::new(GetTaxRateResponse {
                    tax_rate: Some(tax_rate_to_proto(&rate)),
                }))
            }
            None => {
                Err(Status::not_found("Tax rate not found"))
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "ListTaxRates")
    )]
    pub(crate) async fn handle_list_tax_rates(
        &self,
        request: Request<ListTaxRatesRequest>,
    ) -> Result<Response<ListTaxRatesResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let as_of_date = if req.as_of_date.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.as_of_date, "%Y-%m-%d").map_err(|_| {
                    Status::invalid_argument("Invalid as_of_date format")
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
                Status::internal("Failed to list tax rates")
            })?;

        let next_page_token = if tax_rates.len() == page_size as usize {
            tax_rates.last().map(|r| r.tax_rate_id.to_string())
        } else {
            None
        };

        Ok(Response::new(ListTaxRatesResponse {
            tax_rates: tax_rates.iter().map(tax_rate_to_proto).collect(),
            next_page_token: next_page_token.unwrap_or_default(),
        }))
    }

    #[instrument(
        skip(self, request),
        fields(service = "invoicing-service", method = "UpdateTaxRate")
    )]
    pub(crate) async fn handle_update_tax_rate(
        &self,
        request: Request<UpdateTaxRateRequest>,
    ) -> Result<Response<UpdateTaxRateResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id).map_err(|_| {
            Status::invalid_argument("Invalid tenant_id format")
        })?;

        let tax_rate_id = Uuid::parse_str(&req.tax_rate_id).map_err(|_| {
            Status::invalid_argument("Invalid tax_rate_id format")
        })?;

        let rate = if req.rate.is_empty() {
            None
        } else {
            Some(Decimal::from_str(&req.rate).map_err(|_| {
                Status::invalid_argument("Invalid rate format")
            })?)
        };

        let effective_from = if req.effective_from.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_from, "%Y-%m-%d").map_err(|_| {
                    Status::invalid_argument("Invalid effective_from format")
                })?,
            )
        };

        let effective_to = if req.effective_to.is_empty() {
            None
        } else {
            Some(
                NaiveDate::parse_from_str(&req.effective_to, "%Y-%m-%d").map_err(|_| {
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
                Status::internal("Failed to update tax rate")
            })?;

        match tax_rate {
            Some(rate) => {
                Ok(Response::new(UpdateTaxRateResponse {
                    tax_rate: Some(tax_rate_to_proto(&rate)),
                }))
            }
            None => {
                Err(Status::not_found("Tax rate not found"))
            }
        }
    }
}
