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
    models::{Client, ClientType, ServiceAccount},
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
    pub signing_secret: String,
    pub app_name: String,
    pub app_type: ClientType,
}

#[derive(Debug, Serialize)]
pub struct RotateSecretResponse {
    pub client_id: String,
    pub new_client_secret: String,
    pub new_signing_secret: String,
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

    // Generate signing_secret (32 random bytes, URL-safe base64 encoded)
    let signing_secret = {
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
        signing_secret.clone(),
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
            signing_secret,
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

    // Generate new signing_secret
    let new_signing_secret = {
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
                    "signing_secret": &new_signing_secret,
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
            new_signing_secret,
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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateServiceAccountRequest {
    #[validate(length(min = 1, message = "Service name is required"))]
    pub service_name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateServiceAccountResponse {
    pub service_id: String,
    pub api_key: String,
    pub service_name: String,
    pub scopes: Vec<String>,
}

pub async fn create_service_account(
    State(state): State<AppState>,
    Json(req): Json<CreateServiceAccountRequest>,
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

    // Determine environment prefix
    let prefix = match state.config.environment {
        crate::config::Environment::Prod => "svc_live_",
        crate::config::Environment::Dev => "svc_test_",
    };

    // Generate API key (32 random bytes, URL-safe base64 encoded)
    let random_part = {
        let mut rng = rand::thread_rng();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes);
        URL_SAFE_NO_PAD.encode(key_bytes)
    };
    let api_key = format!("{}{}", prefix, random_part);

    // Hash API key for verification (Argon2)
    let key_hash = hash_password(&Password::new(api_key.clone())).map_err(|e| {
        tracing::error!(error = %e, "Failed to hash API key");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // Calculate deterministic hash for lookup (SHA-256)
    let lookup_hash = ServiceAccount::calculate_lookup_hash(&api_key);

    // Create service account model
    let service_account = ServiceAccount::new(
        req.service_name.clone(),
        key_hash.into_string(),
        lookup_hash,
        req.scopes.clone(),
    );

    let service_id = service_account.service_id.clone();

    // Save to database
    state
        .db
        .service_accounts()
        .insert_one(&service_account, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error creating service account");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(
        service_id = %service_id,
        service_name = %service_account.service_name,
        "New service account registered"
    );

    // Return credentials (api_key shown only once)
    Ok((
        StatusCode::CREATED,
        Json(CreateServiceAccountResponse {
            service_id,
            api_key,
            service_name: service_account.service_name,
            scopes: service_account.scopes,
        }),
    ))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RotateServiceKeyResponse {
    pub service_id: String,
    pub new_api_key: String,
    pub previous_key_expiry: chrono::DateTime<chrono::Utc>,
}

pub async fn rotate_service_key(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Find service account
    let account = state
        .db
        .service_accounts()
        .find_one(doc! { "service_id": &service_id }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding service account");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let account = account.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Service account not found".to_string(),
            }),
        )
    })?;

    // 2. Generate new API key
    let prefix = match state.config.environment {
        crate::config::Environment::Prod => "svc_live_",
        crate::config::Environment::Dev => "svc_test_",
    };

    let random_part = {
        let mut rng = rand::thread_rng();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes);
        URL_SAFE_NO_PAD.encode(key_bytes)
    };
    let new_api_key = format!("{}{}", prefix, random_part);

    // 3. Hash new key
    let new_key_hash = hash_password(&Password::new(new_api_key.clone())).map_err(|e| {
        tracing::error!(error = %e, "Failed to hash new API key");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    let new_lookup_hash = ServiceAccount::calculate_lookup_hash(&new_api_key);

    // 4. Update DB
    let now = chrono::Utc::now();
    let expiry = now + chrono::Duration::days(7);

    state
        .db
        .service_accounts()
        .update_one(
            doc! { "service_id": &service_id },
            doc! {
                "$set": {
                    "api_key_hash": new_key_hash.into_string(),
                    "api_key_lookup_hash": new_lookup_hash,
                    "previous_api_key_hash": account.api_key_hash.clone(),
                    "previous_api_key_lookup_hash": account.api_key_lookup_hash.clone(),
                    "previous_key_expiry": expiry,
                    "updated_at": now
                }
            },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error updating service account");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 5. Clear cache for the OLD key (the one that was just moved to previous)
    // Actually, we should clear it so that the middleware re-fetches and sees it's now a previous key.
    let old_cache_key = format!("svc_auth:{}", account.api_key_lookup_hash);
    let _ = state.redis.set_cache(&old_cache_key, "", 0).await;

    tracing::info!(service_id = %service_id, "Service API key rotated");

    Ok((
        StatusCode::OK,
        Json(RotateServiceKeyResponse {
            service_id,
            new_api_key,
            previous_key_expiry: expiry,
        }),
    ))
}

pub async fn revoke_service_account(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Find to get lookup hashes for cache clearing
    let account = state
        .db
        .service_accounts()
        .find_one(doc! { "service_id": &service_id }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding service account");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let account = account.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Service account not found".to_string(),
            }),
        )
    })?;

    // 2. Clear cache for current and previous keys
    let cache_key = format!("svc_auth:{}", account.api_key_lookup_hash);
    let _ = state.redis.set_cache(&cache_key, "", 0).await;
    if let Some(prev_hash) = account.previous_api_key_lookup_hash {
        let prev_cache_key = format!("svc_auth:{}", prev_hash);
        let _ = state.redis.set_cache(&prev_cache_key, "", 0).await;
    }

    // 3. Update enabled = false
    state
        .db
        .service_accounts()
        .update_one(
            doc! { "service_id": &service_id },
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
            tracing::error!(error = %e, "Database error revoking service account");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(service_id = %service_id, "Service account revoked");

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Service account revoked successfully"
        })),
    ))
}

pub async fn get_service_audit_log(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    use mongodb::options::FindOptions;

    let filter = doc! { "service_id": service_id };
    let find_options = FindOptions::builder()
        .sort(doc! { "timestamp": -1 })
        .limit(100)
        .build();

    let mut cursor = state
        .db
        .audit_logs()
        .find(filter, find_options)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding audit logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let mut logs = Vec::new();
    while cursor.advance().await.map_err(|e| {
        tracing::error!(error = %e, "Cursor error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })? {
        logs.push(cursor.deserialize_current().map_err(|e| {
            tracing::error!(error = %e, "Deserialization error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?);
    }

    Ok((StatusCode::OK, Json(logs)))
}
