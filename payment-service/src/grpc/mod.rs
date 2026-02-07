pub mod capability_check;
pub mod customers;
pub mod helpers;
pub mod linked_accounts;
pub mod payment_links;
pub mod payment_service;
pub mod refunds;
pub mod settlements;
pub mod subscriptions;
pub mod transfers;
pub mod webhooks;

pub use capability_check::{capabilities, CapabilityChecker};
pub use payment_service::PaymentGrpcService;

// Include generated proto code
pub mod proto {
    tonic::include_proto!("micros.payment.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("payment_descriptor");
}
