//! Invoice model for invoicing-service.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Invoice type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceType {
    Standard,
    CreditNote,
    Proforma,
}

impl InvoiceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvoiceType::Standard => "standard",
            InvoiceType::CreditNote => "credit_note",
            InvoiceType::Proforma => "proforma",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "credit_note" => InvoiceType::CreditNote,
            "proforma" => InvoiceType::Proforma,
            _ => InvoiceType::Standard,
        }
    }
}

/// Invoice status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Paid,
    Void,
    Overdue,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Issued => "issued",
            InvoiceStatus::Paid => "paid",
            InvoiceStatus::Void => "void",
            InvoiceStatus::Overdue => "overdue",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "issued" => InvoiceStatus::Issued,
            "paid" => InvoiceStatus::Paid,
            "void" => InvoiceStatus::Void,
            "overdue" => InvoiceStatus::Overdue,
            _ => InvoiceStatus::Draft,
        }
    }
}

/// Invoice document.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub invoice_id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_number: Option<String>,
    pub invoice_type: String,
    pub status: String,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub billing_line1: Option<String>,
    pub billing_line2: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_postal_code: Option<String>,
    pub billing_country: Option<String>,
    pub currency: String,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub total: Decimal,
    pub amount_paid: Decimal,
    pub amount_due: Decimal,
    pub notes: Option<String>,
    pub reference_invoice_id: Option<Uuid>,
    pub journal_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
    pub issued_utc: Option<DateTime<Utc>>,
    pub voided_utc: Option<DateTime<Utc>>,
}

/// Filter parameters for listing invoices.
#[derive(Debug, Clone, Default)]
pub struct ListInvoicesFilter {
    pub status: Option<InvoiceStatus>,
    pub customer_id: Option<uuid::Uuid>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub page_size: i32,
    pub page_token: Option<uuid::Uuid>,
}

/// Input for creating an invoice.
#[derive(Debug, Clone)]
pub struct CreateInvoice {
    pub tenant_id: Uuid,
    pub invoice_type: String,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub billing_line1: Option<String>,
    pub billing_line2: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_postal_code: Option<String>,
    pub billing_country: Option<String>,
    pub currency: String,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub reference_invoice_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Input for updating an invoice (draft only).
#[derive(Debug, Clone, Default)]
pub struct UpdateInvoice {
    pub customer_name: Option<String>,
    pub billing_line1: Option<String>,
    pub billing_line2: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_postal_code: Option<String>,
    pub billing_country: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
