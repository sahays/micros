use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreatePaymentLinkRequest {
    pub amount: u64,
    pub currency: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<PaymentLinkCustomer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_by: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PaymentLinkCustomer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayPaymentLinkResponse {
    pub id: String,
    pub amount: u64,
    pub currency: String,
    pub status: String,
    pub short_url: String,
    pub description: Option<String>,
    pub amount_paid: Option<u64>,
    pub expire_by: Option<u64>,
}

impl RazorpayClient {
    pub async fn create_payment_link(
        &self,
        request: CreatePaymentLinkRequest,
    ) -> Result<RazorpayPaymentLinkResponse> {
        let url = format!("{}/payment_links", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn get_payment_link(
        &self,
        payment_link_id: &str,
    ) -> Result<RazorpayPaymentLinkResponse> {
        let url = format!("{}/payment_links/{}", self.api_base_url(), payment_link_id);
        self.authed_get(&url).await
    }

    pub async fn cancel_payment_link(
        &self,
        payment_link_id: &str,
    ) -> Result<RazorpayPaymentLinkResponse> {
        let url = format!(
            "{}/payment_links/{}/cancel",
            self.api_base_url(),
            payment_link_id
        );
        let empty: serde_json::Value = serde_json::json!({});
        self.authed_post(&url, &empty).await
    }
}
