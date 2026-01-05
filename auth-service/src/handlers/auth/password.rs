use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::net::SocketAddr;

use crate::{
    dtos::{
        auth::{PasswordResetConfirm, PasswordResetRequest},
        ErrorResponse,
    },
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
) -> Result<impl IntoResponse, Response> {
    let base_url = format!("http://localhost:{}", state.config.port);
    state
        .auth_service
        .request_password_reset(req, addr.to_string(), base_url)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
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
) -> Result<impl IntoResponse, Response> {
    state
        .auth_service
        .confirm_password_reset(req, addr.to_string())
        .await
        .map_err(|e| {
            let status = match &e {
                crate::services::ServiceError::InvalidToken
                | crate::services::ServiceError::TokenExpired => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Password reset successful. You can now login with your new password."
        })),
    ))
}
