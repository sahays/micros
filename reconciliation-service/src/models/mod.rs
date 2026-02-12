//! Domain models for reconciliation-service.

#![allow(clippy::should_implement_trait)]

mod adjustment;
mod bank_account;
mod matching_rule;
mod reconciliation;
mod statement;
mod transaction;
mod transaction_match;

pub use adjustment::*;
pub use bank_account::*;
pub use matching_rule::*;
pub use reconciliation::*;
pub use statement::*;
pub use transaction::*;
pub use transaction_match::*;

// ============================================================================
// Utility Functions (shared by all model modules)
// ============================================================================

use chrono::{DateTime, Utc};
use prost_types::Timestamp;

pub(crate) fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
