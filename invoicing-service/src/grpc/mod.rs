//! gRPC module for invoicing-service.

pub mod capability_check;
pub mod helpers;
mod invoice_lifecycle;
mod invoices;
mod line_items;
mod pdf;
mod receipts;
mod service;
mod statements;
mod tax_rates;

pub use service::InvoicingServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.invoicing.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("invoicing_descriptor");
}
