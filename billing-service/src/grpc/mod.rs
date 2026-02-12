//! gRPC module for billing-service.

mod billing_cycles;
mod billing_runs;
pub mod capability_check;
mod charges;
pub mod helpers;
mod plans;
mod service;
mod subscriptions;
mod trace_interceptor;
mod usage;

pub use capability_check::{capabilities, CapabilityChecker};
pub use service::BillingServiceImpl;
pub use trace_interceptor::{extract_trace_context, trace_context_interceptor};

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.billing.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("billing_descriptor");
}
