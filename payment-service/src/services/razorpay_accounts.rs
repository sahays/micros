use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreateAccountRequest {
    pub email: String,
    pub phone: Option<String>,
    #[serde(rename = "type")]
    pub account_type: String,
    pub legal_business_name: String,
    pub business_type: String,
    pub legal_info: Option<CreateAccountLegalInfo>,
}

#[derive(Debug, Serialize)]
pub struct CreateAccountLegalInfo {
    pub pan: Option<String>,
    pub gst: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayAccountResponse {
    pub id: String,
    pub email: String,
    pub status: Option<String>,
    pub legal_business_name: Option<String>,
    pub business_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateAccountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_business_name: Option<String>,
}

impl RazorpayClient {
    pub async fn create_account(
        &self,
        request: CreateAccountRequest,
    ) -> Result<RazorpayAccountResponse> {
        let url = format!("{}/accounts", self.api_base_url_v2());
        self.authed_post(&url, &request).await
    }

    pub async fn get_account(&self, account_id: &str) -> Result<RazorpayAccountResponse> {
        let url = format!("{}/accounts/{}", self.api_base_url_v2(), account_id);
        self.authed_get(&url).await
    }

    pub async fn update_account(
        &self,
        account_id: &str,
        request: UpdateAccountRequest,
    ) -> Result<RazorpayAccountResponse> {
        let url = format!("{}/accounts/{}", self.api_base_url_v2(), account_id);
        self.authed_patch(&url, &request).await
    }
}
