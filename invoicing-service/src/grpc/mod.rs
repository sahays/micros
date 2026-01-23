//! gRPC module for invoicing-service.

pub mod capability_check;
mod service;

pub use service::InvoicingServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.invoicing.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("invoicing_descriptor");
}
