//! Domain models for the GenAI service.

pub mod session;
pub mod usage;

pub use session::{Session, SessionDocument, SessionMessage};
pub use usage::{UsageRecord, UsageStats};
