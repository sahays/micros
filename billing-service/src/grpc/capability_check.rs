//! Capability definitions for billing-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{AuthContext, CapabilityChecker};

/// Billing service capabilities.
pub mod capabilities {
    /// Create billing plans.
    pub const BILLING_PLAN_CREATE: &str = "billing.plan:create";

    /// Read billing plans.
    pub const BILLING_PLAN_READ: &str = "billing.plan:read";

    /// Update billing plans.
    pub const BILLING_PLAN_UPDATE: &str = "billing.plan:update";

    /// Create subscriptions.
    pub const BILLING_SUBSCRIPTION_CREATE: &str = "billing.subscription:create";

    /// Read subscriptions.
    pub const BILLING_SUBSCRIPTION_READ: &str = "billing.subscription:read";

    /// Manage subscriptions (cancel, suspend, resume).
    pub const BILLING_SUBSCRIPTION_MANAGE: &str = "billing.subscription:manage";

    /// Change subscriptions (upgrade/downgrade).
    pub const BILLING_SUBSCRIPTION_CHANGE: &str = "billing.subscription:change";

    /// Write usage records.
    pub const BILLING_USAGE_WRITE: &str = "billing.usage:write";

    /// Read usage records.
    pub const BILLING_USAGE_READ: &str = "billing.usage:read";

    /// Read billing cycles.
    pub const BILLING_CYCLE_READ: &str = "billing.cycle:read";

    /// Manage billing cycles.
    pub const BILLING_CYCLE_MANAGE: &str = "billing.cycle:manage";

    /// Create charges.
    pub const BILLING_CHARGE_CREATE: &str = "billing.charge:create";

    /// Execute billing runs.
    pub const BILLING_RUN_EXECUTE: &str = "billing.run:execute";

    /// Read billing runs.
    pub const BILLING_RUN_READ: &str = "billing.run:read";
}
