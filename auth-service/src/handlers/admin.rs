use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use mongodb::bson::doc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    models::{Client, ClientType},
    utils::{hash_password, Password},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateClientRequest {
    #[validate(length(min = 1, message = "App name is required"))]
    pub app_name: String,

    pub app_type: ClientType,

    #[validate(range(min = 1, message = "Rate limit must be at least 1"))]
    pub rate_limit_per_min: u32,

    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateClientResponse {
    pub client_id: String,
    pub client_secret: String,
    pub app_name: String,
    pub app_type: ClientType,
}

#[derive(Debug, Serialize)]
pub struct RotateSecretResponse {
    pub client_id: String,
    pub new_client_secret: String,
    pub previous_secret_expiry: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn create_client(
    State(state): State<AppState>,
    Json(req): Json<CreateClientRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        )
    })?;

    // Generate client_id (UUID v4)
    let client_id = uuid::Uuid::new_v4().to_string();

    // Generate client_secret (32 random bytes, URL-safe base64 encoded)
    let client_secret = {
        let mut rng = rand::thread_rng();
        let mut secret_bytes = [0u8; 32];
        rng.fill(&mut secret_bytes);
        URL_SAFE_NO_PAD.encode(secret_bytes)
    };

    // Hash client_secret
    let secret_hash = hash_password(&Password::new(client_secret.clone())).map_err(|e| {
        tracing::error!(error = %e, "Failed to hash client secret");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // Create client model
    let client = Client::new(
        client_id.clone(),
        secret_hash.into_string(),
        req.app_name.clone(),
        req.app_type.clone(),
        req.rate_limit_per_min,
        req.allowed_origins,
    );

    // Save to database
    state
        .db
        .clients()
        .insert_one(&client, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error creating client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(
        client_id = %client.client_id,
        app_name = %client.app_name,
        "New client registered"
    );

    // Return credentials (secret shown only once)
    Ok((
        StatusCode::CREATED,
        Json(CreateClientResponse {
            client_id,
            client_secret,
            app_name: client.app_name,
            app_type: client.app_type,
        }),
    ))
}

pub async fn rotate_client_secret(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Find client
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &client_id }, None)
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
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Client not found".to_string(),
            }),
        )
    })?;

    // 2. Generate new secret
    let new_client_secret = {
        let mut rng = rand::thread_rng();
        let mut secret_bytes = [0u8; 32];
        rng.fill(&mut secret_bytes);
        URL_SAFE_NO_PAD.encode(secret_bytes)
    };

    // 3. Hash new secret
    let new_secret_hash =
        hash_password(&Password::new(new_client_secret.clone())).map_err(|e| {
            tracing::error!(error = %e, "Failed to hash new client secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 4. Update DB
    let now = chrono::Utc::now();
    let expiry = now + chrono::Duration::hours(24);

    state
        .db
        .clients()
        .update_one(
            doc! { "client_id": &client_id },
            doc! {
                "$set": {
                    "client_secret_hash": new_secret_hash.into_string(),
                    "previous_client_secret_hash": client.client_secret_hash,
                    "previous_secret_expiry": expiry,
                    "updated_at": now
                }
            },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error updating client secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(client_id = %client_id, "Client secret rotated");

    Ok((
        StatusCode::OK,
        Json(RotateSecretResponse {
            client_id,
            new_client_secret,
            previous_secret_expiry: expiry,
        }),
    ))
}

pub async fn revoke_client(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Update enabled = false
    let result = state
        .db
        .clients()
        .update_one(
            doc! { "client_id": &client_id },
            doc! {
                "$set": {
                    "enabled": false,
                    "updated_at": mongodb::bson::DateTime::from_chrono(chrono::Utc::now())
                }
            },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error revoking client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if result.matched_count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Client not found".to_string(),
            }),
        ));
    }

    // 2. Blacklist client_id in Redis (prefixed to avoid collision with JTI)
    // Revoke for 1 hour (max app token duration)
    let blacklist_key = format!("client:{}", client_id);
    state
        .redis
        .blacklist_token(&blacklist_key, 3600)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to blacklist client");
            // Non-fatal, but logged
        })
        .ok();

    tracing::info!(client_id = %client_id, "Client revoked");

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Client revoked successfully"
        })),
    ))
}
