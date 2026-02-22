use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RazorpayPlan {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub razorpay_plan_id: String,
    pub name: String,
    pub description: String,
    pub amount: u64,
    pub currency: String,
    pub period: PlanPeriod,
    pub interval: i32,
    pub created_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanPeriod {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RazorpaySubscription {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub razorpay_subscription_id: String,
    pub plan_id: String,
    pub customer_id: Option<String>,
    pub status: SubscriptionStatus,
    pub total_count: i32,
    pub paid_count: i32,
    pub remaining_count: i32,
    pub short_url: Option<String>,
    pub charge_count: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubscriptionStatus {
    Created,
    Authenticated,
    Active,
    Pending,
    Halted,
    Paused,
    Cancelled,
    Completed,
    Expired,
}
