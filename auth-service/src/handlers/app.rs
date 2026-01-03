use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    services::TokenResponse,
    utils::{verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct AppTokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

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
