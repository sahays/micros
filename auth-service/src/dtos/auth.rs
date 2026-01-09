use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    #[schema(example = "user@example.com")]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[schema(example = "password123", min_length = 8)]
    pub password: String,

    #[schema(example = "John Doe")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub user_id: String,
    #[schema(example = "Registration successful. Please check your email to verify your account.")]
    pub message: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
pub struct VerifyRequest {
    #[validate(length(min = 1, message = "Token is required"))]
    #[schema(example = "abc123token")]
    #[param(example = "abc123token")]
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyResponse {
    #[schema(example = "Email verified successfully")]
    pub message: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    #[schema(example = "user@example.com")]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    #[schema(example = "password123")]
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    #[schema(example = "refresh-token-123")]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    #[schema(example = "refresh-token-123")]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct IntrospectRequest {
    #[schema(example = "access-token-123")]
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntrospectResponse {
    #[schema(example = true)]
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "user@example.com")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 1704326400)]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 1704322800)]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "jti-uuid")]
    pub jti: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PasswordResetRequest {
    #[validate(email(message = "Invalid email format"))]
    #[schema(example = "user@example.com")]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PasswordResetConfirm {
    #[schema(example = "a1b2c3d4e5f6...")]
    pub token: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[schema(example = "newpassword123", min_length = 8)]
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: String,
    pub state: String,
}
