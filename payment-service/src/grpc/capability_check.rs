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

    // Linked account capabilities
    pub const PAYMENT_LINKED_ACCOUNT_CREATE: &str = "payment.linked_account:create";
    pub const PAYMENT_LINKED_ACCOUNT_READ: &str = "payment.linked_account:read";
    pub const PAYMENT_LINKED_ACCOUNT_UPDATE: &str = "payment.linked_account:update";
    pub const PAYMENT_COMMISSION_MANAGE: &str = "payment.commission:manage";

    // Customer capabilities
    pub const PAYMENT_CUSTOMER_CREATE: &str = "payment.customer:create";
    pub const PAYMENT_CUSTOMER_READ: &str = "payment.customer:read";
    pub const PAYMENT_CUSTOMER_UPDATE: &str = "payment.customer:update";

    // Transfer capabilities
    pub const PAYMENT_TRANSFER_CREATE: &str = "payment.transfer:create";
    pub const PAYMENT_TRANSFER_READ: &str = "payment.transfer:read";
    pub const PAYMENT_TRANSFER_REVERSE: &str = "payment.transfer:reverse";
    pub const PAYMENT_TRANSFER_HOLD: &str = "payment.transfer:hold";

    // Settlement capabilities
    pub const PAYMENT_SETTLEMENT_CREATE: &str = "payment.settlement:create";
    pub const PAYMENT_SETTLEMENT_READ: &str = "payment.settlement:read";

    // Plan & subscription capabilities
    pub const PAYMENT_PLAN_CREATE: &str = "payment.plan:create";
    pub const PAYMENT_PLAN_READ: &str = "payment.plan:read";
    pub const PAYMENT_SUBSCRIPTION_CREATE: &str = "payment.subscription:create";
    pub const PAYMENT_SUBSCRIPTION_READ: &str = "payment.subscription:read";
    pub const PAYMENT_SUBSCRIPTION_MANAGE: &str = "payment.subscription:manage";

    // Payment link capabilities
    pub const PAYMENT_LINK_CREATE: &str = "payment.payment_link:create";
    pub const PAYMENT_LINK_READ: &str = "payment.payment_link:read";
    pub const PAYMENT_LINK_CANCEL: &str = "payment.payment_link:cancel";

    // Refund capabilities
    pub const PAYMENT_REFUND_CREATE: &str = "payment.refund:create";
    pub const PAYMENT_REFUND_READ: &str = "payment.refund:read";
}
