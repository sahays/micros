//! Capability definitions for invoicing-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Invoicing service capabilities.
pub mod capabilities {
    /// Create invoices.
    pub const INVOICE_CREATE: &str = "invoicing.invoice:create";

    /// Read invoices.
    pub const INVOICE_READ: &str = "invoicing.invoice:read";

    /// Update invoices.
    pub const INVOICE_UPDATE: &str = "invoicing.invoice:update";

    /// Delete invoices.
    pub const INVOICE_DELETE: &str = "invoicing.invoice:delete";

    /// Issue invoices (finalize and send).
    pub const INVOICE_ISSUE: &str = "invoicing.invoice:issue";

    /// Void invoices.
    pub const INVOICE_VOID: &str = "invoicing.invoice:void";

    /// Record payments.
    pub const PAYMENT_RECORD: &str = "invoicing.payment:record";

    /// Read payments.
    pub const PAYMENT_READ: &str = "invoicing.payment:read";

    /// Create tax rates.
    pub const TAX_RATE_CREATE: &str = "invoicing.tax_rate:create";

    /// Read tax rates.
    pub const TAX_RATE_READ: &str = "invoicing.tax_rate:read";

    /// Update tax rates.
    pub const TAX_RATE_UPDATE: &str = "invoicing.tax_rate:update";

    /// Read customer statements.
    pub const STATEMENT_READ: &str = "invoicing.statement:read";
}
