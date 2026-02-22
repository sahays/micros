use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkedAccount {
    #[serde(rename = "_id")]
    pub id: String,
    pub app_id: String,
    pub tenant_id: String,
    pub razorpay_account_id: String,
    pub name: String,
    pub email: String,
    pub status: LinkedAccountStatus,
    pub commission: Option<CommissionConfig>,
    pub bank_account: Option<BankAccount>,
    pub legal_info: Option<LegalInfo>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinkedAccountStatus {
    Created,
    UnderReview,
    NeedsClarification,
    Activated,
    Suspended,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommissionConfig {
    pub commission_type: CommissionType,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommissionType {
    Flat,
    Percentage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BankAccount {
    pub account_holder_name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub account_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LegalInfo {
    pub legal_business_name: String,
    pub business_type: String,
    pub pan: Option<String>,
    pub gst: Option<String>,
}
