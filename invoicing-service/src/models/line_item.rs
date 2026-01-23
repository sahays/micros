//! Line item model for invoicing-service.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Line item on an invoice.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LineItem {
    pub line_item_id: Uuid,
    pub invoice_id: Uuid,
    pub tenant_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate_id: Option<Uuid>,
    pub tax_amount: Decimal,
    pub subtotal: Decimal,
    pub total: Decimal,
    pub ledger_account_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_utc: DateTime<Utc>,
}

/// Input for creating a line item.
#[derive(Debug, Clone)]
pub struct CreateLineItem {
    pub tenant_id: Uuid,
    pub invoice_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate_id: Option<Uuid>,
    pub ledger_account_id: Option<Uuid>,
    pub sort_order: i32,
}

/// Input for updating a line item.
#[derive(Debug, Clone)]
pub struct UpdateLineItem {
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub tax_rate_id: Option<Uuid>,
    pub ledger_account_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}
