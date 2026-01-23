//! Tax rate model for invoicing-service.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Tax rate configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaxRate {
    pub tax_rate_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub rate: Decimal,
    pub calculation: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub active: bool,
    pub created_utc: DateTime<Utc>,
}

/// Input for creating a tax rate.
#[derive(Debug, Clone)]
pub struct CreateTaxRate {
    pub tenant_id: Uuid,
    pub name: String,
    pub rate: Decimal,
    pub calculation: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

/// Input for updating a tax rate.
#[derive(Debug, Clone)]
pub struct UpdateTaxRate {
    pub name: Option<String>,
    pub rate: Option<Decimal>,
    pub calculation: Option<String>,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub active: Option<bool>,
}
