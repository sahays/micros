use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use std::time::Duration;

use crate::config::GmailConfig;

#[derive(Clone)]
pub struct EmailService {
    mailer: SmtpTransport,
    from_email: String,
}

impl EmailService {
    pub fn new(config: &GmailConfig) -> Result<Self, anyhow::Error> {
        let creds = Credentials::new(config.user.clone(), config.app_password.clone());

        let mailer = SmtpTransport::relay("smtp.gmail.com")?
            .credentials(creds)
            .port(587)
            .timeout(Some(Duration::from_secs(10)))
            .build();

        tracing::info!("Email service initialized with Gmail SMTP");

        Ok(Self {
            mailer,
            from_email: config.user.clone(),
        })
    }

    pub async fn send_verification_email(
        &self,
        to_email: &str,
        verification_token: &str,
        base_url: &str,
    ) -> Result<(), anyhow::Error> {
        let verification_link = format!("{}/auth/verify?token={}", base_url, verification_token);

        let html_body = format!(
            r#"
            <html>
                <body style="font-family: Arial, sans-serif;">
                    <h2>Welcome! Please verify your email</h2>
                    <p>Thank you for registering. Please click the link below to verify your email address:</p>
                    <p>
                        <a href="{}" style="background-color: #4CAF50; color: white; padding: 14px 20px; text-decoration: none; border-radius: 4px;">
                            Verify Email
                        </a>
                    </p>
                    <p>Or copy and paste this link into your browser:</p>
                    <p>{}</p>
                    <p style="color: #666; font-size: 12px;">
                        This link will expire in 24 hours. If you didn't request this, please ignore this email.
                    </p>
                </body>
            </html>
            "#,
            verification_link, verification_link
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

    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        reset_token: &str,
        base_url: &str,
    ) -> Result<(), anyhow::Error> {
        let reset_link = format!(
            "{}/auth/password-reset/confirm?token={}",
            base_url, reset_token
        );

        let html_body = format!(
            r#"
            <html>
                <body style="font-family: Arial, sans-serif;">
                    <h2>Password Reset Request</h2>
                    <p>We received a request to reset your password. Click the link below to set a new password:</p>
                    <p>
                        <a href="{}" style="background-color: #2196F3; color: white; padding: 14px 20px; text-decoration: none; border-radius: 4px;">
                            Reset Password
                        </a>
                    </p>
                    <p>Or copy and paste this link into your browser:</p>
                    <p>{}</p>
                    <p style="color: #666; font-size: 12px;">
                        This link will expire in 1 hour. If you didn't request this, please ignore this email.
                    </p>
                </body>
            </html>
            "#,
            reset_link, reset_link
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

    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        plain_body: &str,
        html_body: &str,
    ) -> Result<(), anyhow::Error> {
        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(plain_body.to_string()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )?;

        // Send email in blocking thread pool to avoid blocking async runtime
        let mailer = self.mailer.clone();
        let result = tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to spawn email task: {}", e))?;

        match result {
            Ok(_) => {
                tracing::info!(
                    to = %to_email,
                    subject = %subject,
                    "Email sent successfully"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    to = %to_email,
                    "Failed to send email"
                );
                Err(anyhow::anyhow!("Failed to send email: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_service_creation() {
        let config = GmailConfig {
            user: "test@gmail.com".to_string(),
            app_password: "test_password".to_string(),
        };

        let service = EmailService::new(&config);
        assert!(service.is_ok());
    }

    // Note: Actual email sending tests would require valid Gmail credentials
    // and would send real emails, so they're omitted from unit tests.
    // Integration tests with a test email account can be added separately.
}
