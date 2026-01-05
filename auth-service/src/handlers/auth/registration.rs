use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::net::SocketAddr;

use crate::{
    dtos::{
        auth::{RegisterRequest, VerifyRequest},
        ErrorResponse,
    },
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
) -> Result<impl IntoResponse, Response> {
    let ip_address = addr.to_string();
    let base_url = format!("http://localhost:{}", state.config.port);

    let res = state
        .auth_service
        .register(req, ip_address, base_url)
        .await
        .map_err(|e| {
            let status = match &e {
                crate::services::ServiceError::EmailAlreadyRegistered => StatusCode::CONFLICT,
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
    axum::extract::Query(req): axum::extract::Query<VerifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let res = state
        .auth_service
        .verify_email(req.token)
        .await
        .map_err(|e| {
            let status = match &e {
                crate::services::ServiceError::InvalidToken => StatusCode::NOT_FOUND,
                crate::services::ServiceError::TokenExpired => StatusCode::BAD_REQUEST,
                crate::services::ServiceError::UserNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((StatusCode::OK, Json(res)))
}
