use service_core::axum::async_trait;
use service_core::error::AppError;
use service_core::gmail::{GmailApiClient, GmailApiConfig};

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send_verification_email(
        &self,
        to_email: &str,
        verification_token: &str,
        base_url: &str,
    ) -> Result<(), AppError>;

    async fn send_password_reset_email(
        &self,
        to_email: &str,
        reset_token: &str,
        base_url: &str,
    ) -> Result<(), AppError>;
}

pub struct EmailService {
    gmail_client: GmailApiClient,
}

impl EmailService {
    pub fn new(config: &crate::config::GmailApiConfig) -> Result<Self, AppError> {
        let gmail_config = GmailApiConfig {
            service_account_key_path: config.service_account_key_path.clone(),
            sender_email: config.sender_email.clone(),
            sender_name: config.sender_name.clone(),
            enabled: config.enabled,
        };

        let gmail_client = GmailApiClient::new(&gmail_config).map_err(AppError::InternalError)?;

        tracing::info!("Email service initialized with Gmail API");

        Ok(Self { gmail_client })
    }

    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        plain_body: &str,
        html_body: &str,
    ) -> Result<(), AppError> {
        self.gmail_client
            .send_email(to_email, subject, plain_body, html_body, None, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    to = %to_email,
                    "Failed to send email via Gmail API"
                );
                AppError::EmailError(e.to_string())
            })?;

        tracing::info!(
            to = %to_email,
            subject = %subject,
            "Email sent successfully"
        );

        Ok(())
    }
}

#[async_trait]
impl EmailProvider for EmailService {
    async fn send_verification_email(
        &self,
        to_email: &str,
        verification_token: &str,
        base_url: &str,
    ) -> Result<(), AppError> {
        let verification_link = format!("{}/auth/verify?token={}", base_url, verification_token);

        let html_body = format!(
            r###"            <html>
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
            </html>
            "###,
            verification_link
        );

        let plain_body = format!(
            "Welcome! Please verify your email\n\n            Thank you for registering. Please visit the following link to verify your email address:\n\n            {}

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
            r###"            <html>
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
            </html>
            "###,
            reset_link
        );

        let plain_body = format!(
            "Password Reset Request\n\n            We received a request to reset your password. Please visit the following link to set a new password:\n\n            {}

            This link will expire in 1 hour. If you didn't request this, please ignore this email.",
            reset_link
        );

        self.send_email(to_email, "Reset Your Password", &plain_body, &html_body)
            .await
    }
}

#[derive(Clone)]
pub struct MockEmailService;

#[async_trait]
impl EmailProvider for MockEmailService {
    async fn send_verification_email(
        &self,
        _to_email: &str,
        _verification_token: &str,
        _base_url: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn send_password_reset_email(
        &self,
        _to_email: &str,
        _reset_token: &str,
        _base_url: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_service_creation_fails_without_key_file() {
        let config = crate::config::GmailApiConfig {
            service_account_key_path: "/nonexistent/key.json".to_string(),
            sender_email: "test@example.com".to_string(),
            sender_name: "Test".to_string(),
            enabled: true,
        };

        let service = EmailService::new(&config);
        assert!(service.is_err());
    }
}
