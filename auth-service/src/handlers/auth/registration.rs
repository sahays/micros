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
    dtos::auth::{RegisterRequest, VerifyRequest},
    utils::ValidatedJson,
    AppState,
};

/// Register a new user
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = RegisterResponse),
        (status = 409, description = "Email already registered", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip_address = addr.to_string();
    let base_url = format!("http://localhost:{}", state.config.common.port);

    // TODO(Story #277): Extract app_id and org_id from tenant context middleware
    // For now, use placeholder values for backward compatibility
    let app_id = "00000000-0000-0000-0000-000000000000".to_string();
    let org_id = "00000000-0000-0000-0000-000000000000".to_string();

    let res = state
        .auth_service
        .register(req, app_id, org_id, ip_address, base_url)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, ip = %addr.to_string(), "Failed to register user");
            e
        })?;

    Ok((StatusCode::CREATED, Json(res)))
}

/// Verify user email
#[utoipa::path(
    get,
    path = "/auth/verify",
    params(VerifyRequest),
    responses(
        (status = 200, description = "Email verified successfully", body = VerifyResponse),
        (status = 400, description = "Token expired", body = ErrorResponse),
        (status = 404, description = "Invalid token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn verify_email(
    State(state): State<AppState>,
    service_core::axum::extract::Query(req): service_core::axum::extract::Query<VerifyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let res = state
        .auth_service
        .verify_email(req.token.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, token = %req.token, "Failed to verify email");
            e
        })?;
    Ok((StatusCode::OK, Json(res)))
}
