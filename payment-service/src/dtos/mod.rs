use crate::models::{Transaction, TransactionStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct QrGenerateRequest {
    pub amount: f64,
    pub description: Option<String>,
    pub transaction_id: Option<Uuid>,
    pub vpa: Option<String>,
    pub merchant_name: Option<String>,
}

#[derive(Serialize)]
pub struct QrGenerateResponse {
    pub upi_link: String,
    pub qr_image_base64: Option<String>,
}

/// Request to create a new transaction.
#[derive(Deserialize)]
pub struct CreateTransactionRequest {
    pub amount: f64,
    pub currency: String,
}

/// Response for a transaction.
#[derive(Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub app_id: String,
    pub org_id: String,
    pub user_id: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub status: TransactionStatus,
    pub provider_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Transaction> for TransactionResponse {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            app_id: t.app_id,
            org_id: t.org_id,
            user_id: t.user_id,
            amount: t.amount,
            currency: t.currency,
            status: t.status,
            provider_order_id: t.provider_order_id,
            created_at: t.created_at.to_string(),
            updated_at: t.updated_at.to_string(),
        }
    }
}

/// Request to update transaction status.
#[derive(Deserialize)]
pub struct UpdateTransactionStatusRequest {
    pub status: TransactionStatus,
}
