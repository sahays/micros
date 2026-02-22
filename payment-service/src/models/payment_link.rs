use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentLink {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub razorpay_payment_link_id: String,
    pub amount: u64,
    pub currency: String,
    pub description: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub status: PaymentLinkStatus,
    pub short_url: String,
    pub amount_paid: u64,
    pub expire_by: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentLinkStatus {
    Created,
    PartiallyPaid,
    Paid,
    Cancelled,
    Expired,
}
