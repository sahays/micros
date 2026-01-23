//! Receipt model for invoicing-service.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Payment receipt.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Receipt {
    pub receipt_id: Uuid,
    pub tenant_id: Uuid,
    pub receipt_number: String,
    pub invoice_id: Uuid,
    pub customer_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub payment_method: String,
    pub payment_reference: Option<String>,
    pub payment_date: NaiveDate,
    pub journal_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_utc: DateTime<Utc>,
}

/// Filter parameters for listing receipts.
#[derive(Debug, Clone, Default)]
pub struct ListReceiptsFilter {
    pub invoice_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}

/// Input for recording a payment.
#[derive(Debug, Clone)]
pub struct CreateReceipt {
    pub tenant_id: Uuid,
    pub invoice_id: Uuid,
    pub amount: Decimal,
    pub payment_method: String,
    pub payment_reference: Option<String>,
    pub payment_date: NaiveDate,
    pub journal_id: Option<Uuid>,
    pub notes: Option<String>,
}
