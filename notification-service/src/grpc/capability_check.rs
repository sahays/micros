//! Capability definitions for notification-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Notification service capabilities.
pub mod capabilities {
    /// Send email notifications.
    pub const NOTIFICATION_EMAIL_SEND: &str = "notification.email:send";

    /// Send SMS notifications.
    pub const NOTIFICATION_SMS_SEND: &str = "notification.sms:send";

    /// Send push notifications.
    pub const NOTIFICATION_PUSH_SEND: &str = "notification.push:send";

    /// Send batch notifications.
    pub const NOTIFICATION_BATCH_SEND: &str = "notification.batch:send";

    /// View notifications.
    pub const NOTIFICATION_READ: &str = "notification:read";
}
