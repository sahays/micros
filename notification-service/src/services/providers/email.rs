use super::{EmailMessage, EmailProvider, ProviderError, ProviderResponse};
use crate::config::GmailApiConfig;
use async_trait::async_trait;
use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::Message;
use service_core::gmail;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct GmailApiProvider {
    config: GmailApiConfig,
    client: Option<gmail::GmailApiClient>,
}

impl GmailApiProvider {
    pub fn new(config: GmailApiConfig) -> Result<Self, ProviderError> {
        if !config.enabled {
            return Ok(Self {
                config,
                client: None,
            });
        }

        let gmail_config = gmail::GmailApiConfig {
            service_account_key_path: config.service_account_key_path.clone(),
            sender_email: config.sender_email.clone(),
            sender_name: config.sender_name.clone(),
            enabled: config.enabled,
        };

        let client = gmail::GmailApiClient::new(&gmail_config).map_err(|e| {
            ProviderError::Configuration(format!("Failed to create Gmail API client: {}", e))
        })?;

        Ok(Self {
            config,
            client: Some(client),
        })
    }
}

#[async_trait]
impl EmailProvider for GmailApiProvider {
    async fn send(&self, email: &EmailMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::NotEnabled(
                "Gmail API email provider is not enabled".to_string(),
            ));
        }

        let client = self.client.as_ref().ok_or_else(|| {
            ProviderError::Configuration("Gmail API client not initialized".to_string())
        })?;

        let from_name = email.from_name.as_ref().unwrap_or(&self.config.sender_name);
        let from_mailbox: Mailbox = format!("{} <{}>", from_name, self.config.sender_email)
            .parse()
            .map_err(|e| ProviderError::Configuration(format!("Invalid from address: {}", e)))?;

        let to_mailbox: Mailbox = email
            .to
            .parse()
            .map_err(|e| ProviderError::InvalidRecipient(format!("Invalid recipient: {}", e)))?;

        let mut message_builder = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(&email.subject);

        if let Some(reply_to) = &email.reply_to {
            let reply_mailbox: Mailbox = reply_to.parse().map_err(|e| {
                ProviderError::Configuration(format!("Invalid reply-to address: {}", e))
            })?;
            message_builder = message_builder.reply_to(reply_mailbox);
        }

        let message = match (&email.body_text, &email.body_html) {
            (Some(text), Some(html)) => message_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(text.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html.clone()),
                        ),
                )
                .map_err(|e| {
                    ProviderError::SendFailed(format!("Failed to build message: {}", e))
                })?,
            (Some(text), None) => message_builder
                .header(ContentType::TEXT_PLAIN)
                .body(text.clone())
                .map_err(|e| {
                    ProviderError::SendFailed(format!("Failed to build message: {}", e))
                })?,
            (None, Some(html)) => message_builder
                .header(ContentType::TEXT_HTML)
                .body(html.clone())
                .map_err(|e| {
                    ProviderError::SendFailed(format!("Failed to build message: {}", e))
                })?,
            (None, None) => {
                return Err(ProviderError::SendFailed(
                    "Email must have either text or HTML body".to_string(),
                ));
            }
        };

        client.send_raw_email(&message).await.map_err(|e| {
            ProviderError::SendFailed(format!("Failed to send email via Gmail API: {}", e))
        })?;

        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "Email sent successfully via Gmail API"
        );

        Ok(ProviderResponse::success(None))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Gmail API client is initialized — that's our health check
        if self.client.is_none() {
            return Err(ProviderError::Configuration(
                "Gmail API client not initialized".to_string(),
            ));
        }

        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Mock email provider for testing
pub struct MockEmailProvider {
    enabled: bool,
    send_count: AtomicU64,
}

impl MockEmailProvider {
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
impl EmailProvider for MockEmailProvider {
    async fn send(&self, email: &EmailMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotEnabled(
                "Mock email provider is not enabled".to_string(),
            ));
        }

        self.send_count.fetch_add(1, Ordering::SeqCst);

        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "[MOCK] Email would be sent"
        );

        Ok(ProviderResponse::success(Some(format!(
            "mock-email-{}",
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
