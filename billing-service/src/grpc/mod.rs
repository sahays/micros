//! gRPC module for billing-service.

mod billing_cycles;
mod billing_runs;
mod charges;
pub mod helpers;
mod plans;
mod service;
mod subscriptions;
mod usage;

pub use service::BillingServiceImpl;

/// Generated protobuf code.
pub mod proto {
    tonic::include_proto!("micros.billing.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("billing_descriptor");
}
