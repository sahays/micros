pub mod email;
pub mod push;
pub mod sms;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub use email::{MockEmailProvider, SmtpProvider};
pub use push::{FcmProvider, MockPushProvider};
pub use sms::{MockSmsProvider, Msg91Provider};

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider not enabled: {0}")]
    NotEnabled(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Send error: {0}")]
    SendFailed(String),

    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Authentication error: {0}")]
    Authentication(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub provider_id: Option<String>,
    pub success: bool,
    pub message: Option<String>,
}

impl ProviderResponse {
    pub fn success(provider_id: Option<String>) -> Self {
        Self {
            provider_id,
            success: true,
            message: None,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            provider_id: None,
            success: false,
            message: Some(message),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub from_name: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SmsMessage {
    pub to: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct PushMessage {
    pub device_token: String,
    pub platform: crate::models::PushPlatform,
    pub title: String,
    pub body: String,
    pub data: Option<HashMap<String, String>>,
}

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, email: &EmailMessage) -> Result<ProviderResponse, ProviderError>;
    async fn health_check(&self) -> Result<(), ProviderError>;
    fn is_enabled(&self) -> bool;
}

#[async_trait]
pub trait SmsProvider: Send + Sync {
    async fn send(&self, sms: &SmsMessage) -> Result<ProviderResponse, ProviderError>;
    async fn health_check(&self) -> Result<(), ProviderError>;
    fn is_enabled(&self) -> bool;
}

#[async_trait]
pub trait PushProvider: Send + Sync {
    async fn send(&self, push: &PushMessage) -> Result<ProviderResponse, ProviderError>;
    async fn health_check(&self) -> Result<(), ProviderError>;
    fn is_enabled(&self) -> bool;
}
