//! gRPC module for reconciliation-service.

mod bank_accounts;
mod matching;
mod reconciliation;
mod service;
mod statements;

pub use service::ReconciliationServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.reconciliation.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("reconciliation_descriptor");
}
