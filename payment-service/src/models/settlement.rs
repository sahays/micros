use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settlement {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub razorpay_settlement_id: String,
    pub linked_account_id: Option<String>,
    pub amount: u64,
    pub currency: String,
    pub status: SettlementStatus,
    pub settlement_type: SettlementType,
    pub utr: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementStatus {
    Created,
    Processed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementType {
    Normal,
    OnDemand,
}
