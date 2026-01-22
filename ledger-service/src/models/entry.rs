//! Ledger entry model for double-entry accounting.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Entry direction (debit or credit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Debit,
    Credit,
}

impl Direction {
    /// Convert from proto enum value.
    pub fn from_proto(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Debit),
            2 => Some(Self::Credit),
            _ => None,
        }
    }

    /// Convert to proto enum value.
    pub fn to_proto(self) -> i32 {
        match self {
            Self::Debit => 1,
            Self::Credit => 2,
        }
    }

    /// Get string representation for database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Credit => "credit",
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Single ledger entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: Uuid,
    pub tenant_id: Uuid,
    pub journal_id: Uuid,
    pub account_id: Uuid,
    pub amount: Decimal,
    pub direction: String,
    pub effective_date: NaiveDate,
    pub posted_utc: DateTime<Utc>,
    pub idempotency_key: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl LedgerEntry {
    /// Get parsed direction.
    pub fn parsed_direction(&self) -> Option<Direction> {
        match self.direction.as_str() {
            "debit" => Some(Direction::Debit),
            "credit" => Some(Direction::Credit),
            _ => None,
        }
    }

    /// Get signed amount (positive for debit, negative for credit).
    pub fn signed_amount(&self) -> Decimal {
        match self.parsed_direction() {
            Some(Direction::Debit) => self.amount,
            Some(Direction::Credit) => -self.amount,
            None => Decimal::ZERO,
        }
    }
}

/// Input for posting a single entry in a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostEntry {
    pub account_id: Uuid,
    pub amount: Decimal,
    pub direction: Direction,
}

/// Transaction groups multiple entries by journal_id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub journal_id: Uuid,
    pub tenant_id: Uuid,
    pub entries: Vec<LedgerEntry>,
    pub effective_date: NaiveDate,
    pub posted_utc: DateTime<Utc>,
    pub idempotency_key: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Statement line with running balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementLine {
    pub entry_id: Uuid,
    pub journal_id: Uuid,
    pub effective_date: NaiveDate,
    pub direction: Direction,
    pub amount: Decimal,
    pub running_balance: Decimal,
    pub metadata: Option<serde_json::Value>,
}

/// Account statement for a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub account_id: Uuid,
    pub currency: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub lines: Vec<StatementLine>,
}
