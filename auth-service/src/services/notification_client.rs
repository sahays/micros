//! Notification service client for auth-service.
//!
//! Sends emails and SMS via the notification-service with trace context propagation.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use service_core::axum::async_trait;
use service_core::error::AppError;
use service_core::observability::trace_context::TracedClientExt;
use std::time::Duration;
use tracing::instrument;

use super::EmailProvider;

/// Configuration for the notification service client.
#[derive(Debug, Clone)]
pub struct NotificationClientConfig {
    pub base_url: String,
    pub timeout_seconds: u64,
}

impl Default for NotificationClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://notification-service:8080".to_string(),
            timeout_seconds: 30,
        }
    }
}

/// Client for interacting with the notification-service.
#[derive(Clone)]
pub struct NotificationClient {
    client: Client,
    base_url: String,
}

/// Request to send an email via notification-service.
#[derive(Debug, Serialize)]
struct SendEmailRequest {
    to: String,
    subject: String,
    body_html: String,
    body_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

/// Response from notification-service.
#[derive(Debug, Deserialize)]
struct NotificationResponse {
    notification_id: String,
    status: String,
    #[allow(dead_code)]
    channel: String,
}

impl NotificationClient {
    /// Create a new notification client.
    pub fn new(config: NotificationClientConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("Failed to create HTTP client: {}", e))
            })?;

        tracing::info!(
            base_url = %config.base_url,
            "Notification client initialized"
        );

        Ok(Self {
            client,
            base_url: config.base_url,
        })
    }

    /// Send an email via the notification service.
    #[instrument(skip(self, body_html, body_text), fields(to = %to, subject = %subject))]
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: &str,
    ) -> Result<(), AppError> {
        let url = format!("{}/notifications/email", self.base_url);

        let request = SendEmailRequest {
            to: to.to_string(),
            subject: subject.to_string(),
            body_html: body_html.to_string(),
            body_text: body_text.to_string(),
            from_name: Some("Auth Service".to_string()),
            reply_to: None,
            metadata: None,
        };

        let response = self
            .client
            .traced_post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to send email request to notification service");
                AppError::InternalError(anyhow::anyhow!("Notification service error: {}", e))
            })?;

        if response.status().is_success() {
            let result: NotificationResponse = response.json().await.map_err(|e| {
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to parse notification response: {}",
                    e
                ))
            })?;

            tracing::info!(
                notification_id = %result.notification_id,
                status = %result.status,
                "Email queued successfully"
            );

            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                status = %status,
                error = %error_text,
                "Notification service returned error"
            );
            Err(AppError::InternalError(anyhow::anyhow!(
                "Notification service error: {} - {}",
                status,
                error_text
            )))
        }
    }
}

#[async_trait]
impl EmailProvider for NotificationClient {
    async fn send_verification_email(
        &self,
        to_email: &str,
        verification_token: &str,
        base_url: &str,
    ) -> Result<(), AppError> {
        let verification_link = format!("{}/auth/verify?token={}", base_url, verification_token);

        let html_body = format!(
            r###"<html>
                <body style="font-family: Arial, sans-serif;">
                    <h2>Welcome! Please verify your email</h2>
                    <p>Thank you for registering. Please click the link below to verify your email address:</p>
                    <p>
                        <a href="{}" style="background-color: #4CAF50; color: white; padding: 14px 20px; text-decoration: none; border-radius: 4px;">
                            Verify Email
                        </a>
                    </p>
                    <p style="color: #666; font-size: 12px;">
                        This link will expire in 24 hours. If you didn't request this, please ignore this email.
                    </p>
                </body>
            </html>"###,
            verification_link
        );

        let plain_body = format!(
            "Welcome! Please verify your email\n\n\
            Thank you for registering. Please visit the following link to verify your email address:\n\n\
            {}\n\n\
            This link will expire in 24 hours. If you didn't request this, please ignore this email.",
            verification_link
        );

        self.send_email(
            to_email,
            "Verify Your Email Address",
            &plain_body,
            &html_body,
        )
        .await
    }

    async fn send_password_reset_email(
        &self,
        to_email: &str,
        reset_token: &str,
        base_url: &str,
    ) -> Result<(), AppError> {
        let reset_link = format!(
            "{}/auth/password-reset/confirm?token={}",
            base_url, reset_token
        );

        let html_body = format!(
            r###"<html>
                <body style="font-family: Arial, sans-serif;">
                    <h2>Password Reset Request</h2>
                    <p>We received a request to reset your password. Click the link below to set a new password:</p>
                    <p>
                        <a href="{}" style="background-color: #2196F3; color: white; padding: 14px 20px; text-decoration: none; border-radius: 4px;">
                            Reset Password
                        </a>
                    </p>
                    <p style="color: #666; font-size: 12px;">
                        This link will expire in 1 hour. If you didn't request this, please ignore this email.
                    </p>
                </body>
            </html>"###,
            reset_link
        );

        let plain_body = format!(
            "Password Reset Request\n\n\
            We received a request to reset your password. Please visit the following link to set a new password:\n\n\
            {}\n\n\
            This link will expire in 1 hour. If you didn't request this, please ignore this email.",
            reset_link
        );

        self.send_email(to_email, "Reset Your Password", &plain_body, &html_body)
            .await
    }
}
