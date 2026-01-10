use service_core::{
    axum::{
        extract::{ConnectInfo, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    },
    error::AppError,
};
use std::net::SocketAddr;

use crate::{
    dtos::auth::{PasswordResetConfirm, PasswordResetRequest},
    utils::ValidatedJson,
    AppState,
};

/// Request a password reset link
#[utoipa::path(
    post,
    path = "/auth/password-reset/request",
    request_body = PasswordResetRequest,
    responses(
        (status = 200, description = "Request received"),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn request_password_reset(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(req): ValidatedJson<PasswordResetRequest>,
) -> Result<impl IntoResponse, AppError> {
    let base_url = format!("http://localhost:{}", state.config.common.port);
    state
        .auth_service
        .request_password_reset(req, addr.to_string(), base_url)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, ip = %addr.to_string(), "Failed to process password reset request");
            e
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "If your email is registered, you will receive a password reset link shortly."
        })),
    ))
}

/// Confirm password reset with token
#[utoipa::path(
    post,
    path = "/auth/password-reset/confirm",
    request_body = PasswordResetConfirm,
    responses(
        (status = 200, description = "Password reset successful"),
        (status = 400, description = "Invalid or expired token", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn confirm_password_reset(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(req): ValidatedJson<PasswordResetConfirm>,
) -> Result<impl IntoResponse, AppError> {
    state
        .auth_service
        .confirm_password_reset(req, addr.to_string())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, ip = %addr.to_string(), "Failed to confirm password reset");
            e
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Password reset successful. You can now login with your new password."
        })),
    ))
}
