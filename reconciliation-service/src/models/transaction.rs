use crate::grpc::proto;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Staged,
    Unmatched,
    Matched,
    ManuallyMatched,
    Excluded,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Unmatched => "unmatched",
            Self::Matched => "matched",
            Self::ManuallyMatched => "manually_matched",
            Self::Excluded => "excluded",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "staged" => Self::Staged,
            "unmatched" => Self::Unmatched,
            "matched" => Self::Matched,
            "manually_matched" => Self::ManuallyMatched,
            "excluded" => Self::Excluded,
            _ => Self::Staged,
        }
    }
}

impl From<TransactionStatus> for proto::TransactionStatus {
    fn from(s: TransactionStatus) -> Self {
        match s {
            TransactionStatus::Staged => Self::Staged,
            TransactionStatus::Unmatched => Self::Unmatched,
            TransactionStatus::Matched => Self::Matched,
            TransactionStatus::ManuallyMatched => Self::ManuallyMatched,
            TransactionStatus::Excluded => Self::Excluded,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct BankTransaction {
    pub transaction_id: Uuid,
    pub statement_id: Uuid,
    pub tenant_id: Uuid,
    pub transaction_date: NaiveDate,
    pub description: String,
    pub reference: Option<String>,
    pub amount: Decimal,
    pub running_balance: Option<Decimal>,
    pub status: String,
    pub extraction_confidence: Option<f64>,
    pub is_modified: bool,
    pub created_utc: DateTime<Utc>,
}

impl From<BankTransaction> for proto::StagedTransaction {
    fn from(t: BankTransaction) -> Self {
        Self {
            transaction_id: t.transaction_id.to_string(),
            statement_id: t.statement_id.to_string(),
            tenant_id: t.tenant_id.to_string(),
            transaction_date: t.transaction_date.to_string(),
            description: t.description,
            reference: t.reference,
            amount: t.amount.to_string(),
            running_balance: t.running_balance.map(|b| b.to_string()),
            status: proto::TransactionStatus::from(TransactionStatus::from_str(&t.status)).into(),
            extraction_confidence: t.extraction_confidence,
            is_modified: t.is_modified,
            created_utc: Some(datetime_to_timestamp(t.created_utc)),
        }
    }
}
