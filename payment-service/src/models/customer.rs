use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RazorpayCustomer {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub razorpay_customer_id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
