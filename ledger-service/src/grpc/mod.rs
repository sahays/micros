//! gRPC module for ledger-service.

pub mod capability_check;
mod service;

pub use service::LedgerServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.ledger.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("ledger_descriptor");
}
