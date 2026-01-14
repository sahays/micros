use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "_id")]
    pub id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub status: TransactionStatus,
    pub provider_order_id: Option<String>, // e.g., Razorpay Order ID
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionStatus {
    Created,
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentMethod {
    #[serde(rename = "_id")]
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub is_active: bool,
}
