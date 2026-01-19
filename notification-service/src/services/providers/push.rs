use super::{ProviderError, ProviderResponse, PushMessage, PushProvider};
use crate::config::FcmConfig;
use crate::models::PushPlatform;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const FCM_API_URL: &str = "https://fcm.googleapis.com/v1/projects";

pub struct FcmProvider {
    config: FcmConfig,
    client: Client,
}

#[derive(Debug, Serialize)]
struct FcmRequest {
    message: FcmMessage,
}

#[derive(Debug, Serialize)]
struct FcmMessage {
    token: String,
    notification: FcmNotification,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    android: Option<FcmAndroidConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    apns: Option<FcmApnsConfig>,
}

#[derive(Debug, Serialize)]
struct FcmNotification {
    title: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct FcmAndroidConfig {
    priority: String,
}

#[derive(Debug, Serialize)]
struct FcmApnsConfig {
    headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FcmResponse {
    name: Option<String>,
    #[serde(default)]
    error: Option<FcmError>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FcmError {
    code: i32,
    message: String,
    status: String,
}

impl FcmProvider {
    pub fn new(config: FcmConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        // In production, this would use the service account key to get an OAuth2 token
        // For now, we'll use the service_account_key as a bearer token (for testing)
        // In a real implementation, you'd use google-auth crate or similar
        if self.config.service_account_key.is_empty() {
            return Err(ProviderError::Authentication(
                "FCM service account key not configured".to_string(),
            ));
        }

        // This is a placeholder - real implementation would exchange service account credentials
        // for an OAuth2 access token using Google's OAuth2 endpoint
        Ok(self.config.service_account_key.clone())
    }
}

#[async_trait]
impl PushProvider for FcmProvider {
    async fn send(&self, push: &PushMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::NotEnabled(
                "FCM push provider is not enabled".to_string(),
            ));
        }

        if self.config.project_id.is_empty() {
            return Err(ProviderError::Configuration(
                "FCM project_id is not configured".to_string(),
            ));
        }

        let access_token = self.get_access_token().await?;

        let mut android_config = None;
        let mut apns_config = None;

        match push.platform {
            PushPlatform::Fcm => {
                android_config = Some(FcmAndroidConfig {
                    priority: "high".to_string(),
                });
            }
            PushPlatform::Apns => {
                let mut headers = HashMap::new();
                headers.insert("apns-priority".to_string(), "10".to_string());
                apns_config = Some(FcmApnsConfig { headers });
            }
        }

        let request = FcmRequest {
            message: FcmMessage {
                token: push.device_token.clone(),
                notification: FcmNotification {
                    title: push.title.clone(),
                    body: push.body.clone(),
                },
                data: push.data.clone(),
                android: android_config,
                apns: apns_config,
            },
        };

        let url = format!("{}/{}/messages:send", FCM_API_URL, self.config.project_id);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(format!("Failed to connect to FCM: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::SendFailed(format!(
                "FCM API returned error status {}: {}",
                status, body
            )));
        }

        let fcm_response: FcmResponse = response.json().await.map_err(|e| {
            ProviderError::SendFailed(format!("Failed to parse FCM response: {}", e))
        })?;

        if let Some(error) = fcm_response.error {
            return Err(ProviderError::SendFailed(format!(
                "FCM error ({}): {}",
                error.status, error.message
            )));
        }

        tracing::info!(
            device_token = %push.device_token,
            platform = %push.platform,
            "Push notification sent successfully via FCM"
        );

        Ok(ProviderResponse::success(fcm_response.name))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.config.project_id.is_empty() {
            return Err(ProviderError::Configuration(
                "FCM project_id is not configured".to_string(),
            ));
        }

        if self.config.service_account_key.is_empty() {
            return Err(ProviderError::Configuration(
                "FCM service_account_key is not configured".to_string(),
            ));
        }

        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Mock push provider for testing
pub struct MockPushProvider {
    enabled: bool,
    send_count: AtomicU64,
}

impl MockPushProvider {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            send_count: AtomicU64::new(0),
        }
    }

    pub fn send_count(&self) -> u64 {
        self.send_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PushProvider for MockPushProvider {
    async fn send(&self, push: &PushMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotEnabled(
                "Mock push provider is not enabled".to_string(),
            ));
        }

        self.send_count.fetch_add(1, Ordering::SeqCst);

        tracing::info!(
            device_token = %push.device_token,
            platform = %push.platform,
            title = %push.title,
            "[MOCK] Push notification would be sent"
        );

        Ok(ProviderResponse::success(Some(format!(
            "mock-push-{}",
            self.send_count.load(Ordering::SeqCst)
        ))))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}
