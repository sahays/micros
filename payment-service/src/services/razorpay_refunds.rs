use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreateRefundRequest {
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse_all: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayRefundResponse {
    pub id: String,
    pub entity: String,
    pub amount: u64,
    pub currency: String,
    pub payment_id: Option<String>,
    pub status: String,
    pub speed_requested: Option<String>,
    pub speed_processed: Option<String>,
}

impl RazorpayClient {
    pub async fn create_refund(
        &self,
        payment_id: &str,
        request: CreateRefundRequest,
    ) -> Result<RazorpayRefundResponse> {
        let url = format!("{}/payments/{}/refunds", self.api_base_url(), payment_id);
        self.authed_post(&url, &request).await
    }

    pub async fn get_refund(
        &self,
        payment_id: &str,
        refund_id: &str,
    ) -> Result<RazorpayRefundResponse> {
        let url = format!(
            "{}/payments/{}/refunds/{}",
            self.api_base_url(),
            payment_id,
            refund_id
        );
        self.authed_get(&url).await
    }
}
