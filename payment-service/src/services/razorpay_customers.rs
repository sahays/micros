use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayCustomerResponse {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub contact: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateCustomerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

impl RazorpayClient {
    pub async fn create_customer(
        &self,
        request: CreateCustomerRequest,
    ) -> Result<RazorpayCustomerResponse> {
        let url = format!("{}/customers", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn get_customer(&self, customer_id: &str) -> Result<RazorpayCustomerResponse> {
        let url = format!("{}/customers/{}", self.api_base_url(), customer_id);
        self.authed_get(&url).await
    }

    pub async fn update_customer(
        &self,
        customer_id: &str,
        request: UpdateCustomerRequest,
    ) -> Result<RazorpayCustomerResponse> {
        let url = format!("{}/customers/{}", self.api_base_url(), customer_id);
        self.authed_patch(&url, &request).await
    }
}
