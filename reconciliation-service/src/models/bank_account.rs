use crate::grpc::proto;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, FromRow)]
pub struct BankAccount {
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub ledger_account_id: Uuid,
    pub bank_name: String,
    pub account_number_masked: String,
    pub currency: String,
    pub last_reconciled_date: Option<chrono::NaiveDate>,
    pub last_reconciled_balance: Option<Decimal>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

impl From<BankAccount> for proto::BankAccount {
    fn from(a: BankAccount) -> Self {
        Self {
            bank_account_id: a.bank_account_id.to_string(),
            tenant_id: a.tenant_id.to_string(),
            ledger_account_id: a.ledger_account_id.to_string(),
            bank_name: a.bank_name,
            account_number_masked: a.account_number_masked,
            currency: a.currency,
            last_reconciled_date: a.last_reconciled_date.map(|d| d.to_string()),
            last_reconciled_balance: a.last_reconciled_balance.map(|b| b.to_string()),
            created_utc: Some(datetime_to_timestamp(a.created_utc)),
            updated_utc: Some(datetime_to_timestamp(a.updated_utc)),
        }
    }
}
