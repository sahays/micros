use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use rand::Rng;
use serde::{Deserialize, Serialize};
use validator::Validate;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

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
