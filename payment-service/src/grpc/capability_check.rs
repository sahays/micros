//! Capability definitions for payment-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Payment service capabilities.
pub mod capabilities {
    /// Create transactions.
    pub const PAYMENT_TRANSACTION_CREATE: &str = "payment.transaction:create";

    /// View transactions.
    pub const PAYMENT_TRANSACTION_READ: &str = "payment.transaction:read";

    /// Update transaction status.
    pub const PAYMENT_TRANSACTION_UPDATE: &str = "payment.transaction:update";

    /// Create Razorpay orders.
    pub const PAYMENT_RAZORPAY_CREATE: &str = "payment.razorpay:create";

    /// Verify Razorpay payments.
    pub const PAYMENT_RAZORPAY_VERIFY: &str = "payment.razorpay:verify";

    /// Generate UPI QR codes.
    pub const PAYMENT_UPI_GENERATE: &str = "payment.upi:generate";

    /// Handle payment webhooks.
    pub const PAYMENT_WEBHOOK_HANDLE: &str = "payment.webhook:handle";
}
