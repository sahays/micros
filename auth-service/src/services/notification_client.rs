//! Notification service gRPC client for auth-service.
//!
//! Sends emails and SMS via the notification-service using gRPC.

use service_core::axum::async_trait;
use service_core::error::AppError;
use service_core::grpc::NotificationClient as GrpcNotificationClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use super::EmailProvider;

/// Client for interacting with the notification-service via gRPC.
#[derive(Clone)]
pub struct NotificationClient {
    /// The inner gRPC client, wrapped in RwLock for interior mutability.
    inner: Arc<RwLock<Option<GrpcNotificationClient>>>,
    /// The gRPC endpoint.
    endpoint: String,
}

impl NotificationClient {
    /// Create a new notification client.
    ///
    /// Connection is lazy - it will be established on first use.
    pub fn new(endpoint: &str) -> Self {
        tracing::info!(
            endpoint = %endpoint,
            "Notification gRPC client configured (lazy connection)"
        );

        Self {
            inner: Arc::new(RwLock::new(None)),
            endpoint: endpoint.to_string(),
        }
    }

    /// Get or create the inner gRPC client.
    async fn get_client(&self) -> Result<GrpcNotificationClient, AppError> {
        // First, try to read existing client
        {
            let guard = self.inner.read().await;
            if let Some(client) = guard.as_ref() {
                return Ok(client.clone());
            }
        }

        // Need to create a new client
        let mut guard = self.inner.write().await;

        // Double-check in case another task created it while we were waiting
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }

        // Create new client
        let client = GrpcNotificationClient::connect(&self.endpoint)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, endpoint = %self.endpoint, "Failed to connect to notification service");
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to connect to notification service: {}",
                    e
                ))
            })?;

        tracing::info!(endpoint = %self.endpoint, "Connected to notification gRPC service");
        *guard = Some(client.clone());
        Ok(client)
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
        let mut client = self.get_client().await?;

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "auth-service".to_string());

        let response = client
            .send_email(
                to.to_string(),
                subject.to_string(),
                Some(body_text.to_string()),
                Some(body_html.to_string()),
                Some("Auth Service".to_string()),
                None,
                metadata,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to send email via notification service");
                AppError::InternalError(anyhow::anyhow!("Notification service error: {}", e))
            })?;

        tracing::info!(
            notification_id = %response.notification_id,
            status = %response.status,
            "Email queued successfully via gRPC"
        );

        Ok(())
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
