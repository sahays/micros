//! Capability definitions for ledger-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Ledger service capabilities.
pub mod capabilities {
    /// Create ledger accounts.
    pub const LEDGER_ACCOUNT_CREATE: &str = "ledger.account:create";

    /// Read ledger accounts.
    pub const LEDGER_ACCOUNT_READ: &str = "ledger.account:read";

    /// Update ledger accounts.
    pub const LEDGER_ACCOUNT_UPDATE: &str = "ledger.account:update";

    /// Create transactions (journal entries).
    pub const LEDGER_TRANSACTION_CREATE: &str = "ledger.transaction:create";

    /// Read transactions.
    pub const LEDGER_TRANSACTION_READ: &str = "ledger.transaction:read";

    /// Read account balances.
    pub const LEDGER_BALANCE_READ: &str = "ledger.balance:read";

    /// Read account statements.
    pub const LEDGER_STATEMENT_READ: &str = "ledger.statement:read";

    /// Reverse transactions.
    pub const LEDGER_TRANSACTION_REVERSE: &str = "ledger.transaction:reverse";
}
