//! Account model for double-entry ledger.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Account types following standard accounting categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl AccountType {
    /// Convert from proto enum value.
    pub fn from_proto(value: i32) -> Option<Self> {
        match value {
            1 => Some(Self::Asset),
            2 => Some(Self::Liability),
            3 => Some(Self::Equity),
            4 => Some(Self::Revenue),
            5 => Some(Self::Expense),
            _ => None,
        }
    }

    /// Convert to proto enum value.
    pub fn to_proto(self) -> i32 {
        match self {
            Self::Asset => 1,
            Self::Liability => 2,
            Self::Equity => 3,
            Self::Revenue => 4,
            Self::Expense => 5,
        }
    }

    /// Get string representation for database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Asset => "asset",
            Self::Liability => "liability",
            Self::Equity => "equity",
            Self::Revenue => "revenue",
            Self::Expense => "expense",
        }
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Ledger account.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Account {
    pub account_id: Uuid,
    pub tenant_id: Uuid,
    pub account_type: String,
    pub account_code: String,
    pub currency: String,
    pub allow_negative: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
    pub closed_utc: Option<DateTime<Utc>>,
}

impl Account {
    /// Check if account is closed.
    pub fn is_closed(&self) -> bool {
        self.closed_utc.is_some()
    }

    /// Get parsed account type.
    pub fn parsed_type(&self) -> Option<AccountType> {
        match self.account_type.as_str() {
            "asset" => Some(AccountType::Asset),
            "liability" => Some(AccountType::Liability),
            "equity" => Some(AccountType::Equity),
            "revenue" => Some(AccountType::Revenue),
            "expense" => Some(AccountType::Expense),
            _ => None,
        }
    }
}

/// Account with calculated balance (for queries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountWithBalance {
    pub account: Account,
    pub balance: Decimal,
}

/// Input for creating a new account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub tenant_id: Uuid,
    pub account_type: AccountType,
    pub account_code: String,
    pub currency: String,
    pub allow_negative: bool,
    pub metadata: Option<serde_json::Value>,
}
