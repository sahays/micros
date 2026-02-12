//! gRPC module for ledger-service.

mod accounts;
pub mod capability_check;
mod reporting;
mod service;
mod transactions;

pub use capability_check::CapabilityChecker;
pub use service::LedgerServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.ledger.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("ledger_descriptor");
}
