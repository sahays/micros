use service_core::axum::async_trait;
use service_core::error::AppError;

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

    async fn send_otp_email(
        &self,
        to_email: &str,
        code: &str,
        subject: &str,
        plain_body: &str,
        html_body: &str,
        app_name: Option<&str>,
    ) -> Result<(), AppError>;
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

    async fn send_otp_email(
        &self,
        _to_email: &str,
        _code: &str,
        _subject: &str,
        _plain_body: &str,
        _html_body: &str,
        _app_name: Option<&str>,
    ) -> Result<(), AppError> {
        Ok(())
    }
}
