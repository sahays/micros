use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transfer {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub org_id: String,
    pub razorpay_transfer_id: String,
    pub payment_id: Option<String>,
    pub order_id: Option<String>,
    pub linked_account_id: String,
    pub amount: u64,
    pub currency: String,
    pub status: TransferStatus,
    pub on_hold: bool,
    pub on_hold_until: Option<DateTime>,
    pub commission_amount: u64,
    pub amount_reversed: u64,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferStatus {
    Created,
    Pending,
    Processed,
    Reversed,
    Failed,
}
