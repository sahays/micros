//! Helper and conversion functions for invoicing-service gRPC handlers.

use crate::grpc::proto::{
    Address, Invoice as ProtoInvoice, InvoiceStatus as ProtoInvoiceStatus,
    InvoiceType as ProtoInvoiceType, LineItem as ProtoLineItem, Receipt as ProtoReceipt,
    TaxCalculation, TaxRate as ProtoTaxRate,
};
use crate::models::{Invoice, LineItem, Receipt, TaxRate};
use prost_types::Timestamp;
use rust_decimal::Decimal;

/// Format a Decimal as a normalized string.
pub fn format_decimal(d: &Decimal) -> String {
    let s = d.to_string();
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Convert domain TaxRate to proto TaxRate.
pub fn tax_rate_to_proto(rate: &TaxRate) -> ProtoTaxRate {
    ProtoTaxRate {
        tax_rate_id: rate.tax_rate_id.to_string(),
        tenant_id: rate.tenant_id.to_string(),
        name: rate.name.clone(),
        rate: format_decimal(&rate.rate),
        calculation: match rate.calculation.as_str() {
            "inclusive" => TaxCalculation::Inclusive as i32,
            _ => TaxCalculation::Exclusive as i32,
        },
        effective_from: rate.effective_from.to_string(),
        effective_to: rate.effective_to.map(|d| d.to_string()).unwrap_or_default(),
        active: rate.active,
        created_at: Some(Timestamp {
            seconds: rate.created_utc.timestamp(),
            nanos: rate.created_utc.timestamp_subsec_nanos() as i32,
        }),
    }
}

/// Convert domain LineItem to proto LineItem.
pub fn line_item_to_proto(item: &LineItem) -> ProtoLineItem {
    ProtoLineItem {
        line_item_id: item.line_item_id.to_string(),
        invoice_id: item.invoice_id.to_string(),
        description: item.description.clone(),
        quantity: format_decimal(&item.quantity),
        unit_price: format_decimal(&item.unit_price),
        tax_rate_id: item
            .tax_rate_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        tax_amount: format_decimal(&item.tax_amount),
        subtotal: format_decimal(&item.subtotal),
        total: format_decimal(&item.total),
        ledger_account_id: item
            .ledger_account_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        sort_order: item.sort_order,
    }
}

/// Compute the effective status of an invoice, checking for overdue condition.
pub fn compute_invoice_status(invoice: &Invoice) -> i32 {
    // If invoice is issued and has a due date that's past, it's overdue
    if invoice.status == "issued" {
        if let Some(due_date) = invoice.due_date {
            let today = chrono::Utc::now().date_naive();
            if due_date < today && invoice.amount_due > Decimal::ZERO {
                return ProtoInvoiceStatus::Overdue as i32;
            }
        }
        return ProtoInvoiceStatus::Issued as i32;
    }

    match invoice.status.as_str() {
        "paid" => ProtoInvoiceStatus::Paid as i32,
        "void" => ProtoInvoiceStatus::Void as i32,
        "overdue" => ProtoInvoiceStatus::Overdue as i32,
        _ => ProtoInvoiceStatus::Draft as i32,
    }
}

/// Convert domain Invoice to proto Invoice.
pub fn invoice_to_proto(invoice: &Invoice, line_items: &[LineItem]) -> ProtoInvoice {
    ProtoInvoice {
        invoice_id: invoice.invoice_id.to_string(),
        tenant_id: invoice.tenant_id.to_string(),
        invoice_number: invoice.invoice_number.clone().unwrap_or_default(),
        invoice_type: match invoice.invoice_type.as_str() {
            "credit_note" => ProtoInvoiceType::CreditNote as i32,
            "proforma" => ProtoInvoiceType::Proforma as i32,
            _ => ProtoInvoiceType::Standard as i32,
        },
        status: compute_invoice_status(invoice),
        customer_id: invoice.customer_id.to_string(),
        customer_name: invoice.customer_name.clone(),
        billing_address: Some(Address {
            line1: invoice.billing_line1.clone().unwrap_or_default(),
            line2: invoice.billing_line2.clone().unwrap_or_default(),
            city: invoice.billing_city.clone().unwrap_or_default(),
            state: invoice.billing_state.clone().unwrap_or_default(),
            postal_code: invoice.billing_postal_code.clone().unwrap_or_default(),
            country: invoice.billing_country.clone().unwrap_or_default(),
        }),
        currency: invoice.currency.clone(),
        issue_date: invoice
            .issue_date
            .map(|d| d.to_string())
            .unwrap_or_default(),
        due_date: invoice.due_date.map(|d| d.to_string()).unwrap_or_default(),
        line_items: line_items.iter().map(line_item_to_proto).collect(),
        subtotal: format_decimal(&invoice.subtotal),
        tax_total: format_decimal(&invoice.tax_total),
        total: format_decimal(&invoice.total),
        amount_paid: format_decimal(&invoice.amount_paid),
        amount_due: format_decimal(&invoice.amount_due),
        notes: invoice.notes.clone().unwrap_or_default(),
        reference_invoice_id: invoice
            .reference_invoice_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        journal_id: invoice
            .journal_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        metadata: invoice
            .metadata
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_default(),
        created_at: Some(Timestamp {
            seconds: invoice.created_utc.timestamp(),
            nanos: invoice.created_utc.timestamp_subsec_nanos() as i32,
        }),
        issued_at: invoice.issued_utc.map(|t| Timestamp {
            seconds: t.timestamp(),
            nanos: t.timestamp_subsec_nanos() as i32,
        }),
        voided_at: invoice.voided_utc.map(|t| Timestamp {
            seconds: t.timestamp(),
            nanos: t.timestamp_subsec_nanos() as i32,
        }),
    }
}

/// Convert domain Receipt to proto Receipt.
pub fn receipt_to_proto(receipt: &Receipt) -> ProtoReceipt {
    ProtoReceipt {
        receipt_id: receipt.receipt_id.to_string(),
        tenant_id: receipt.tenant_id.to_string(),
        receipt_number: receipt.receipt_number.clone(),
        invoice_id: receipt.invoice_id.to_string(),
        customer_id: receipt.customer_id.to_string(),
        amount: format_decimal(&receipt.amount),
        currency: receipt.currency.clone(),
        payment_method: receipt.payment_method.clone(),
        payment_reference: receipt.payment_reference.clone().unwrap_or_default(),
        payment_date: receipt.payment_date.to_string(),
        journal_id: receipt
            .journal_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        notes: receipt.notes.clone().unwrap_or_default(),
        created_at: Some(Timestamp {
            seconds: receipt.created_utc.timestamp(),
            nanos: receipt.created_utc.timestamp_subsec_nanos() as i32,
        }),
    }
}

/// Convert DateTime to proto Timestamp.
pub fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
