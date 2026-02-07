use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::razorpay::RazorpayClient;

#[derive(Debug, Serialize)]
pub struct CreatePlanRequest {
    pub period: String,
    pub interval: i32,
    pub item: PlanItem,
}

#[derive(Debug, Serialize)]
pub struct PlanItem {
    pub name: String,
    pub description: String,
    pub amount: u64,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayPlanResponse {
    pub id: String,
    pub entity: String,
    pub interval: Option<i32>,
    pub period: Option<String>,
    pub item: Option<RazorpayPlanItemResponse>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayPlanItemResponse {
    pub name: Option<String>,
    pub description: Option<String>,
    pub amount: Option<u64>,
    pub currency: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSubscriptionRequest {
    pub plan_id: String,
    pub total_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RazorpaySubscriptionResponse {
    pub id: String,
    pub entity: String,
    pub plan_id: String,
    pub status: String,
    pub total_count: Option<i32>,
    pub paid_count: Option<i32>,
    pub remaining_count: Option<i32>,
    pub short_url: Option<String>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateSubscriptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i32>,
}

impl RazorpayClient {
    pub async fn create_plan(&self, request: CreatePlanRequest) -> Result<RazorpayPlanResponse> {
        let url = format!("{}/plans", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn get_plan(&self, plan_id: &str) -> Result<RazorpayPlanResponse> {
        let url = format!("{}/plans/{}", self.api_base_url(), plan_id);
        self.authed_get(&url).await
    }

    pub async fn create_subscription(
        &self,
        request: CreateSubscriptionRequest,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!("{}/subscriptions", self.api_base_url());
        self.authed_post(&url, &request).await
    }

    pub async fn get_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!("{}/subscriptions/{}", self.api_base_url(), subscription_id);
        self.authed_get(&url).await
    }

    pub async fn pause_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!(
            "{}/subscriptions/{}/pause",
            self.api_base_url(),
            subscription_id
        );
        let empty: serde_json::Value = serde_json::json!({});
        self.authed_post(&url, &empty).await
    }

    pub async fn resume_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!(
            "{}/subscriptions/{}/resume",
            self.api_base_url(),
            subscription_id
        );
        let empty: serde_json::Value = serde_json::json!({});
        self.authed_post(&url, &empty).await
    }

    pub async fn cancel_subscription(
        &self,
        subscription_id: &str,
        cancel_at_cycle_end: bool,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!(
            "{}/subscriptions/{}/cancel",
            self.api_base_url(),
            subscription_id
        );
        let body = serde_json::json!({ "cancel_at_cycle_end": cancel_at_cycle_end });
        self.authed_post(&url, &body).await
    }

    pub async fn update_subscription(
        &self,
        subscription_id: &str,
        request: UpdateSubscriptionRequest,
    ) -> Result<RazorpaySubscriptionResponse> {
        let url = format!("{}/subscriptions/{}", self.api_base_url(), subscription_id);
        self.authed_patch(&url, &request).await
    }
}
