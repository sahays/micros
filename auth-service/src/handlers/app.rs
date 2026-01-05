use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mongodb::bson::doc;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    dtos::ErrorResponse,
    services::TokenResponse,
    utils::{verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AppTokenRequest {
    #[schema(example = "client-uuid")]
    pub client_id: String,
    #[schema(example = "client-secret-123")]
    pub client_secret: String,
    #[schema(example = "client_credentials")]
    pub grant_type: String,
}

/// Get a service-to-service app token
#[utoipa::path(
    post,
    path = "/auth/app/token",
    request_body = AppTokenRequest,
    responses(
        (status = 200, description = "App token issued successfully", body = TokenResponse),
        (status = 400, description = "Unsupported grant type", body = ErrorResponse),
        (status = 401, description = "Invalid client credentials", body = ErrorResponse),
        (status = 403, description = "Client is disabled", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Service Authentication"
)]
pub async fn app_token(
    State(state): State<AppState>,
    Json(req): Json<AppTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 0. Validate grant_type
    if req.grant_type != "client_credentials" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "unsupported_grant_type".to_string(),
            }),
        ));
    }

    // 1. Find client
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &req.client_id }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let client = client.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid client_id or client_secret".to_string(),
            }),
        )
    })?;

    // 2. Check if enabled
    if !client.enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Client is disabled".to_string(),
            }),
        ));
    }

    // 3. Verify secret
    // Check current hash
    let mut verified = verify_password(
        &Password::new(req.client_secret.clone()),
        &PasswordHashString::new(client.client_secret_hash.clone()),
    )
    .is_ok();

    // If failed, check previous hash (rotation grace period)
    if !verified {
        if let (Some(prev_hash), Some(prev_expiry)) = (
            &client.previous_client_secret_hash,
            client.previous_secret_expiry,
        ) {
            let now = chrono::Utc::now();
            if now < prev_expiry {
                verified = verify_password(
                    &Password::new(req.client_secret.clone()),
                    &PasswordHashString::new(prev_hash.clone()),
                )
                .is_ok();
            }
        }
    }

    if !verified {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid client_id or client_secret".to_string(),
            }),
        ));
    }

    // 4. Generate App Token
    let token = state
        .jwt
        .generate_app_token(
            &client.client_id,
            &client.app_name,
            vec![],
            client.rate_limit_per_min,
        )
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate app token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(
        client_id = %client.client_id,
        "App token issued"
    );

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            access_token: token,
            refresh_token: "".to_string(), // App flows typically don't use refresh tokens, or use rotation
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
        }),
    ))
}
