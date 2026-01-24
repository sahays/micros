//! Capability definitions for reconciliation-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Reconciliation service capabilities.
pub mod capabilities {
    /// Register bank accounts for reconciliation.
    pub const RECONCILIATION_BANK_ACCOUNT_CREATE: &str = "reconciliation.bank_account:create";

    /// View bank accounts.
    pub const RECONCILIATION_BANK_ACCOUNT_READ: &str = "reconciliation.bank_account:read";

    /// Update bank account configuration.
    pub const RECONCILIATION_BANK_ACCOUNT_UPDATE: &str = "reconciliation.bank_account:update";

    /// Import bank statements.
    pub const RECONCILIATION_STATEMENT_IMPORT: &str = "reconciliation.statement:import";

    /// View bank statements.
    pub const RECONCILIATION_STATEMENT_READ: &str = "reconciliation.statement:read";

    /// Update staged transactions (corrections).
    pub const RECONCILIATION_STAGED_UPDATE: &str = "reconciliation.staged:update";

    /// Commit staged statement transactions.
    pub const RECONCILIATION_STATEMENT_COMMIT: &str = "reconciliation.statement:commit";

    /// Abandon staged statement.
    pub const RECONCILIATION_STATEMENT_ABANDON: &str = "reconciliation.statement:abandon";

    /// Create matching rules.
    pub const RECONCILIATION_RULE_CREATE: &str = "reconciliation.rule:create";

    /// View matching rules.
    pub const RECONCILIATION_RULE_READ: &str = "reconciliation.rule:read";

    /// Update matching rules.
    pub const RECONCILIATION_RULE_UPDATE: &str = "reconciliation.rule:update";

    /// Delete matching rules.
    pub const RECONCILIATION_RULE_DELETE: &str = "reconciliation.rule:delete";

    /// View bank transactions.
    pub const RECONCILIATION_TRANSACTION_READ: &str = "reconciliation.transaction:read";

    /// Match transactions manually.
    pub const RECONCILIATION_MATCH_CREATE: &str = "reconciliation.match:create";

    /// Unmatch transactions.
    pub const RECONCILIATION_MATCH_DELETE: &str = "reconciliation.match:delete";

    /// Exclude transactions from matching.
    pub const RECONCILIATION_EXCLUDE: &str = "reconciliation.transaction:exclude";

    /// Request AI suggestions.
    pub const RECONCILIATION_AI_SUGGEST: &str = "reconciliation.ai:suggest";

    /// Confirm AI suggestions.
    pub const RECONCILIATION_AI_CONFIRM: &str = "reconciliation.ai:confirm";

    /// Start reconciliation process.
    pub const RECONCILIATION_START: &str = "reconciliation.process:start";

    /// View reconciliation status.
    pub const RECONCILIATION_READ: &str = "reconciliation.process:read";

    /// Complete reconciliation.
    pub const RECONCILIATION_COMPLETE: &str = "reconciliation.process:complete";

    /// Abandon reconciliation.
    pub const RECONCILIATION_ABANDON: &str = "reconciliation.process:abandon";

    /// Create adjustment entries.
    pub const RECONCILIATION_ADJUSTMENT_CREATE: &str = "reconciliation.adjustment:create";

    /// View adjustment entries.
    pub const RECONCILIATION_ADJUSTMENT_READ: &str = "reconciliation.adjustment:read";
}
