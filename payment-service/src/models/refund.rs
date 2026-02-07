use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Refund {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub org_id: String,
    pub razorpay_refund_id: String,
    pub payment_id: String,
    pub amount: u64,
    pub currency: String,
    pub status: RefundStatus,
    pub speed: RefundSpeed,
    pub reason: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Created,
    Processed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundSpeed {
    Normal,
    Optimum,
}
