use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreateOnDemandSettlementRequest {
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settle_full_balance: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpaySettlementResponse {
    pub id: String,
    pub entity: String,
    pub amount: u64,
    pub status: String,
    pub utr: Option<String>,
    pub created_at: Option<u64>,
}

impl RazorpayClient {
    pub async fn create_on_demand_settlement(
        &self,
        request: CreateOnDemandSettlementRequest,
    ) -> Result<RazorpaySettlementResponse> {
        let url = format!("{}/settlements/ondemand", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn get_settlement(&self, settlement_id: &str) -> Result<RazorpaySettlementResponse> {
        let url = format!("{}/settlements/{}", self.api_base_url(), settlement_id);
        self.authed_get(&url).await
    }
}
