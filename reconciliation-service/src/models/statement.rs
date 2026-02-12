use crate::grpc::proto;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementStatus {
    Uploaded,
    Extracting,
    Staged,
    Committed,
    Reconciling,
    Reconciled,
    Failed,
    Abandoned,
}

impl StatementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploaded => "uploaded",
            Self::Extracting => "extracting",
            Self::Staged => "staged",
            Self::Committed => "committed",
            Self::Reconciling => "reconciling",
            Self::Reconciled => "reconciled",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "uploaded" => Self::Uploaded,
            "extracting" => Self::Extracting,
            "staged" => Self::Staged,
            "committed" => Self::Committed,
            "reconciling" => Self::Reconciling,
            "reconciled" => Self::Reconciled,
            "failed" => Self::Failed,
            "abandoned" => Self::Abandoned,
            _ => Self::Uploaded,
        }
    }
}

impl From<StatementStatus> for proto::StatementStatus {
    fn from(s: StatementStatus) -> Self {
        match s {
            StatementStatus::Uploaded => Self::Uploaded,
            StatementStatus::Extracting => Self::Extracting,
            StatementStatus::Staged => Self::Staged,
            StatementStatus::Committed => Self::Committed,
            StatementStatus::Reconciling => Self::Reconciling,
            StatementStatus::Reconciled => Self::Reconciled,
            StatementStatus::Failed => Self::Failed,
            StatementStatus::Abandoned => Self::Abandoned,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct BankStatement {
    pub statement_id: Uuid,
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub document_id: Option<Uuid>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub status: String,
    pub error_message: Option<String>,
    pub extraction_confidence: Option<f64>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

impl From<BankStatement> for proto::BankStatement {
    fn from(s: BankStatement) -> Self {
        Self {
            statement_id: s.statement_id.to_string(),
            bank_account_id: s.bank_account_id.to_string(),
            tenant_id: s.tenant_id.to_string(),
            document_id: s.document_id.map(|d| d.to_string()),
            period_start: s.period_start.to_string(),
            period_end: s.period_end.to_string(),
            opening_balance: s.opening_balance.to_string(),
            closing_balance: s.closing_balance.to_string(),
            status: proto::StatementStatus::from(StatementStatus::from_str(&s.status)).into(),
            error_message: s.error_message,
            extraction_confidence: s.extraction_confidence,
            created_utc: Some(datetime_to_timestamp(s.created_utc)),
            updated_utc: Some(datetime_to_timestamp(s.updated_utc)),
        }
    }
}
