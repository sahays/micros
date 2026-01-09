use mongodb::bson::doc;
use service_core::{
    axum::{extract::State, http::StatusCode, response::IntoResponse, Json},
    error::AppError,
    serde::Deserialize,
    validator::Validate,
};
use utoipa::ToSchema;

use crate::{
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
) -> Result<impl IntoResponse, AppError> {
    // 0. Validate grant_type
    if req.grant_type != "client_credentials" {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "unsupported_grant_type"
        )));
    }

    // 1. Find client
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &req.client_id }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding client");
            AppError::InternalError(anyhow::anyhow!("Internal server error"))
        })?
        .ok_or_else(|| {
            AppError::Unauthorized(anyhow::anyhow!("Invalid client_id or client_secret"))
        })?;

    // 2. Check if enabled
    if !client.enabled {
        return Err(AppError::Forbidden(anyhow::anyhow!("Client is disabled")));
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
        return Err(AppError::Unauthorized(anyhow::anyhow!(
            "Invalid client_id or client_secret"
        )));
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
            AppError::InternalError(anyhow::anyhow!("Internal server error"))
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
