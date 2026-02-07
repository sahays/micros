pub mod customer;
pub mod linked_account;
pub mod payment_link;
pub mod refund;
pub mod settlement;
pub mod subscription;
pub mod transfer;

use mongodb::bson::{self, DateTime};
use serde::{Deserialize, Deserializer, Serialize};

pub use customer::RazorpayCustomer;
pub use linked_account::{
    BankAccount, CommissionConfig, CommissionType, LegalInfo, LinkedAccount, LinkedAccountStatus,
};
pub use payment_link::{PaymentLink, PaymentLinkStatus};
pub use refund::{Refund, RefundSpeed, RefundStatus};
pub use settlement::{Settlement, SettlementStatus, SettlementType};
pub use subscription::{PlanPeriod, RazorpayPlan, RazorpaySubscription, SubscriptionStatus};
pub use transfer::{Transfer, TransferStatus};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "_id")]
    pub id: String,
    /// Application ID (maps to registered client in auth-service)
    pub app_id: String,
    /// Organization ID within the application
    pub org_id: String,
    /// User who initiated this transaction (if applicable)
    pub user_id: Option<String>,
    #[serde(
        alias = "amount",
        rename = "amount_paise",
        deserialize_with = "deserialize_amount"
    )]
    pub amount_paise: u64,
    pub currency: String,
    pub status: TransactionStatus,
    pub provider_order_id: Option<String>, // e.g., Razorpay Order ID
    #[serde(default)]
    pub linked_account_id: Option<String>,
    #[serde(default)]
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub payment_link_id: Option<String>,
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
    PartiallyRefunded,
}

/// Deserialize amount from BSON, handling both Double (legacy) and Int64/Int32 values.
/// Legacy documents stored amount as f64 rupees; new documents store u64 paise.
/// Double values are treated as rupees and converted to paise for backward compatibility.
fn deserialize_amount<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let bson_value = bson::Bson::deserialize(deserializer)?;
    match bson_value {
        bson::Bson::Double(v) => Ok((v * 100.0).round() as u64),
        bson::Bson::Int64(v) => Ok(v as u64),
        bson::Bson::Int32(v) => Ok(v as u64),
        other => Err(serde::de::Error::custom(format!(
            "expected numeric type for amount, got {:?}",
            other
        ))),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentMethod {
    #[serde(rename = "_id")]
    pub id: String,
    /// Application ID (maps to registered client in auth-service)
    pub app_id: String,
    /// Organization ID within the application
    pub org_id: String,
    pub name: String,
    pub provider: String,
    pub is_active: bool,
}
