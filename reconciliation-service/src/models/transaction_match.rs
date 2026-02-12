use crate::grpc::proto;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, FromRow)]
pub struct TransactionMatch {
    pub match_id: Uuid,
    pub bank_transaction_id: Uuid,
    pub ledger_entry_id: Uuid,
    pub match_method: String,
    pub confidence_score: Option<f64>,
    pub matched_by: Option<String>,
    pub matched_utc: DateTime<Utc>,
}

impl From<TransactionMatch> for proto::TransactionMatch {
    fn from(m: TransactionMatch) -> Self {
        Self {
            match_id: m.match_id.to_string(),
            bank_transaction_id: m.bank_transaction_id.to_string(),
            ledger_entry_id: m.ledger_entry_id.to_string(),
            match_method: m.match_method,
            confidence_score: m.confidence_score,
            matched_by: m.matched_by,
            matched_utc: Some(datetime_to_timestamp(m.matched_utc)),
        }
    }
}
