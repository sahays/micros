use super::{EmailMessage, EmailProvider, ProviderError, ProviderResponse};
use crate::config::SmtpConfig;
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct SmtpProvider {
    config: SmtpConfig,
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl SmtpProvider {
    pub fn new(config: SmtpConfig) -> Result<Self, ProviderError> {
        if !config.enabled {
            return Ok(Self {
                config,
                transport: None,
            });
        }

        let creds = Credentials::new(config.user.clone(), config.password.clone());

        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .map_err(|e| {
                ProviderError::Configuration(format!("Failed to create SMTP relay: {}", e))
            })?
            .port(config.port)
            .credentials(creds)
            .build();

        Ok(Self {
            config,
            transport: Some(transport),
        })
    }
}

#[async_trait]
impl EmailProvider for SmtpProvider {
    async fn send(&self, email: &EmailMessage) -> Result<ProviderResponse, ProviderError> {
        if !self.config.enabled {
            return Err(ProviderError::NotEnabled(
                "SMTP email provider is not enabled".to_string(),
            ));
        }

        let transport = self.transport.as_ref().ok_or_else(|| {
            ProviderError::Configuration("SMTP transport not initialized".to_string())
        })?;

        let from_name = email.from_name.as_ref().unwrap_or(&self.config.from_name);
        let from_mailbox: Mailbox = format!("{} <{}>", from_name, self.config.from_email)
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

        let response = transport
            .send(message)
            .await
            .map_err(|e| ProviderError::SendFailed(format!("Failed to send email: {}", e)))?;

        let provider_id = response.message().next().map(|s| s.to_string());

        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "Email sent successfully"
        );

        Ok(ProviderResponse::success(provider_id))
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if !self.config.enabled {
            return Ok(());
        }

        let transport = self.transport.as_ref().ok_or_else(|| {
            ProviderError::Configuration("SMTP transport not initialized".to_string())
        })?;

        transport.test_connection().await.map_err(|e| {
            ProviderError::Connection(format!("SMTP connection test failed: {}", e))
        })?;

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
