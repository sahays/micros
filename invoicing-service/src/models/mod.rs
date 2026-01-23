//! Domain models for invoicing-service.

mod invoice;
mod line_item;
mod receipt;
mod tax_rate;

pub use invoice::{
    CreateInvoice, Invoice, InvoiceStatus, InvoiceType, ListInvoicesFilter, UpdateInvoice,
};
pub use line_item::{CreateLineItem, LineItem, UpdateLineItem};
pub use receipt::{CreateReceipt, ListReceiptsFilter, Receipt};
pub use tax_rate::{CreateTaxRate, TaxRate, UpdateTaxRate};
