use crate::grpc::proto;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStatus {
    InProgress,
    Completed,
    Abandoned,
}

impl ReconciliationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "abandoned" => Self::Abandoned,
            _ => Self::InProgress,
        }
    }
}

impl From<ReconciliationStatus> for proto::ReconciliationStatus {
    fn from(s: ReconciliationStatus) -> Self {
        match s {
            ReconciliationStatus::InProgress => Self::InProgress,
            ReconciliationStatus::Completed => Self::Completed,
            ReconciliationStatus::Abandoned => Self::Abandoned,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Reconciliation {
    pub reconciliation_id: Uuid,
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub expected_balance: Decimal,
    pub actual_balance: Decimal,
    pub difference: Decimal,
    pub status: String,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub started_utc: DateTime<Utc>,
    pub completed_utc: Option<DateTime<Utc>>,
}

impl From<Reconciliation> for proto::Reconciliation {
    fn from(r: Reconciliation) -> Self {
        Self {
            reconciliation_id: r.reconciliation_id.to_string(),
            bank_account_id: r.bank_account_id.to_string(),
            tenant_id: r.tenant_id.to_string(),
            period_start: r.period_start.to_string(),
            period_end: r.period_end.to_string(),
            expected_balance: r.expected_balance.to_string(),
            actual_balance: r.actual_balance.to_string(),
            difference: r.difference.to_string(),
            status: proto::ReconciliationStatus::from(ReconciliationStatus::from_str(&r.status))
                .into(),
            matched_count: r.matched_count,
            unmatched_count: r.unmatched_count,
            started_utc: Some(datetime_to_timestamp(r.started_utc)),
            completed_utc: r.completed_utc.map(datetime_to_timestamp),
        }
    }
}
