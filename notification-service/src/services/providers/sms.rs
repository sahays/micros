use super::{ProviderError, ProviderResponse, SmsMessage, SmsProvider};
use crate::config::Msg91Config;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

const MSG91_API_URL: &str = "https://api.msg91.com/api/v5/flow/";

pub struct Msg91Provider {
    config: Msg91Config,
    client: Client,
}

#[derive(Debug, Serialize)]
struct Msg91Request {
    sender: String,
    route: String,
    country: String,
    sms: Vec<Msg91Sms>,
}

#[derive(Debug, Serialize)]
struct Msg91Sms {
    message: String,
    to: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Msg91Response {
    #[serde(rename = "type")]
    response_type: String,
    message: String,
    #[serde(default)]
    request_id: Option<String>,
}

impl Msg91Provider {
    pub fn new(config: Msg91Config) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl SmsProvider for Msg91Provider {
    async fn send(&self, sms: &SmsMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::NotEnabled(
                "Msg91 SMS provider is not enabled".to_string(),
            ));
        }

        // Normalize phone number (remove non-digits except leading +)
        let normalized_phone = sms
            .to
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect::<String>();

        if normalized_phone.is_empty() {
            return Err(ProviderError::InvalidRecipient(
                "Phone number is empty".to_string(),
            ));
        }

        let request = Msg91Request {
            sender: self.config.sender_id.clone(),
            route: "4".to_string(),    // Transactional route
            country: "91".to_string(), // Default to India, can be made configurable
            sms: vec![Msg91Sms {
                message: sms.body.clone(),
                to: vec![normalized_phone.clone()],
            }],
        };

        let response = self
            .client
            .post(MSG91_API_URL)
            .header("authkey", &self.config.auth_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Connection(format!("Failed to connect to Msg91: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::SendFailed(format!(
                "Msg91 API returned error status {}: {}",
                status, body
            )));
        }

        let msg91_response: Msg91Response = response.json().await.map_err(|e| {
            ProviderError::SendFailed(format!("Failed to parse Msg91 response: {}", e))
        })?;

        if msg91_response.response_type != "success" {
            return Err(ProviderError::SendFailed(format!(
                "Msg91 error: {}",
                msg91_response.message
            )));
        }

        tracing::info!(
            to = %sms.to,
            "SMS sent successfully via Msg91"
        );

        Ok(ProviderResponse::success(msg91_response.request_id))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Msg91 doesn't have a dedicated health endpoint, so we just check if config is valid
        if self.config.auth_key.is_empty() {
            return Err(ProviderError::Configuration(
                "Msg91 auth_key is not configured".to_string(),
            ));
        }

        if self.config.sender_id.is_empty() {
            return Err(ProviderError::Configuration(
                "Msg91 sender_id is not configured".to_string(),
            ));
        }

        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Mock SMS provider for testing
pub struct MockSmsProvider {
    enabled: bool,
    send_count: AtomicU64,
}

impl MockSmsProvider {
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
impl SmsProvider for MockSmsProvider {
    async fn send(&self, sms: &SmsMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotEnabled(
                "Mock SMS provider is not enabled".to_string(),
            ));
        }

        self.send_count.fetch_add(1, Ordering::SeqCst);

        tracing::info!(
            to = %sms.to,
            body_length = %sms.body.len(),
            "[MOCK] SMS would be sent"
        );

        Ok(ProviderResponse::success(Some(format!(
            "mock-sms-{}",
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
