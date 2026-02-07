use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreatePaymentTransferRequest {
    pub transfers: Vec<TransferItem>,
}

#[derive(Debug, Serialize)]
pub struct TransferItem {
    pub account: String,
    pub amount: u64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_hold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_hold_until: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateDirectTransferRequest {
    pub account: String,
    pub amount: u64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct ReverseTransferRequest {
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct UpdateTransferHoldRequest {
    pub on_hold: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_hold_until: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayTransferResponse {
    pub id: String,
    pub entity: String,
    pub amount: u64,
    pub currency: String,
    pub status: String,
    pub source: Option<String>,
    pub recipient: Option<String>,
    pub on_hold: Option<bool>,
    pub on_hold_until: Option<u64>,
    pub amount_reversed: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TransferCollectionResponse {
    pub items: Vec<RazorpayTransferResponse>,
}

impl RazorpayClient {
    pub async fn create_payment_transfer(
        &self,
        payment_id: &str,
        request: CreatePaymentTransferRequest,
    ) -> Result<TransferCollectionResponse> {
        let url = format!("{}/payments/{}/transfers", self.api_base_url(), payment_id);
        self.authed_post(&url, &request).await
    }

    pub async fn create_direct_transfer(
        &self,
        request: CreateDirectTransferRequest,
    ) -> Result<RazorpayTransferResponse> {
        let url = format!("{}/transfers", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn reverse_transfer(
        &self,
        transfer_id: &str,
        request: ReverseTransferRequest,
    ) -> Result<RazorpayTransferResponse> {
        let url = format!(
            "{}/transfers/{}/reversals",
            self.api_base_url(),
            transfer_id
        );
        self.authed_post(&url, &request).await
    }

    pub async fn get_transfer(&self, transfer_id: &str) -> Result<RazorpayTransferResponse> {
        let url = format!("{}/transfers/{}", self.api_base_url(), transfer_id);
        self.authed_get(&url).await
    }

    pub async fn update_transfer_hold(
        &self,
        transfer_id: &str,
        request: UpdateTransferHoldRequest,
    ) -> Result<RazorpayTransferResponse> {
        let url = format!("{}/transfers/{}", self.api_base_url(), transfer_id);
        self.authed_patch(&url, &request).await
    }
}
