use service_core::{axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
}, error::AppError};
use std::net::SocketAddr;

use crate::{
    dtos::auth::{IntrospectRequest, LoginRequest, LogoutRequest, RefreshRequest},
    middleware::AuthUser,
    utils::ValidatedJson,
    AppState,
};

/// Login with email and password
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let res = state
        .auth_service
        .login(req, state.config.jwt.refresh_token_expiry_days)
        .await?;
    Ok((StatusCode::OK, Json(res)))
}

/// Logout and invalidate tokens
#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Invalid token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    user: AuthUser,
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    let access_token_claims = user.0;
    state
        .auth_service
        .logout(
            req.refresh_token,
            access_token_claims.jti,
            access_token_claims.exp,
            addr.to_string(),
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Logged out successfully"
        })),
    ))
}

/// Refresh access token using refresh token
#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = TokenResponse),
        (status = 401, description = "Invalid or expired token", body = ErrorResponse),
        (status = 403, description = "User not verified", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Authentication"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    let res = state.auth_service.refresh(req).await?;
    Ok((StatusCode::OK, Json(res)))
}

/// Introspect an access token
#[utoipa::path(
    post,
    path = "/auth/introspect",
    request_body = IntrospectRequest,
    responses(
        (status = 200, description = "Token status returned", body = IntrospectResponse)
    ),
    tag = "Authentication"
)]
pub async fn introspect(
    State(state): State<AppState>,
    Json(req): Json<IntrospectRequest>,
) -> impl IntoResponse {
    let res = state.auth_service.introspect(req.token).await;
    Json(res)
}
