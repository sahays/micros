//! gRPC module for reconciliation-service.

mod bank_accounts;
pub mod capability_check;
mod matching;
mod reconciliation;
mod service;
mod statements;
mod trace_interceptor;

pub use capability_check::{capabilities, CapabilityChecker};
pub use service::ReconciliationServiceImpl;
pub use trace_interceptor::{extract_trace_context, trace_context_interceptor};

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.reconciliation.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("reconciliation_descriptor");
}
