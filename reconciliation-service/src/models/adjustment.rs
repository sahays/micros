use crate::grpc::proto;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentType {
    BankFee,
    BankInterest,
    Correction,
    TimingDifference,
    Other,
}

impl AdjustmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BankFee => "bank_fee",
            Self::BankInterest => "bank_interest",
            Self::Correction => "correction",
            Self::TimingDifference => "timing_difference",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "bank_fee" => Self::BankFee,
            "bank_interest" => Self::BankInterest,
            "correction" => Self::Correction,
            "timing_difference" => Self::TimingDifference,
            "other" => Self::Other,
            _ => Self::Other,
        }
    }

    pub fn from_proto(p: proto::AdjustmentType) -> Self {
        match p {
            proto::AdjustmentType::BankFee => Self::BankFee,
            proto::AdjustmentType::BankInterest => Self::BankInterest,
            proto::AdjustmentType::Correction => Self::Correction,
            proto::AdjustmentType::TimingDifference => Self::TimingDifference,
            proto::AdjustmentType::Other => Self::Other,
            _ => Self::Other,
        }
    }
}

impl From<AdjustmentType> for proto::AdjustmentType {
    fn from(a: AdjustmentType) -> Self {
        match a {
            AdjustmentType::BankFee => Self::BankFee,
            AdjustmentType::BankInterest => Self::BankInterest,
            AdjustmentType::Correction => Self::Correction,
            AdjustmentType::TimingDifference => Self::TimingDifference,
            AdjustmentType::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Adjustment {
    pub adjustment_id: Uuid,
    pub reconciliation_id: Uuid,
    pub tenant_id: Uuid,
    pub adjustment_type: String,
    pub description: String,
    pub amount: Decimal,
    pub ledger_entry_id: Option<Uuid>,
    pub created_utc: DateTime<Utc>,
}

impl From<Adjustment> for proto::Adjustment {
    fn from(a: Adjustment) -> Self {
        Self {
            adjustment_id: a.adjustment_id.to_string(),
            reconciliation_id: a.reconciliation_id.to_string(),
            tenant_id: a.tenant_id.to_string(),
            adjustment_type: proto::AdjustmentType::from(AdjustmentType::from_str(
                &a.adjustment_type,
            ))
            .into(),
            description: a.description,
            amount: a.amount.to_string(),
            ledger_entry_id: a.ledger_entry_id.map(|id| id.to_string()),
            created_utc: Some(datetime_to_timestamp(a.created_utc)),
        }
    }
}
