use service_core::{axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::Next,
    response::Response,
    async_trait,
}, error::AppError};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::{
    models::{AuditLog, ServiceAccount},
    utils::{verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceContext {
    pub service_id: String,
    pub service_name: String,
    pub scopes: Vec<String>,
}

pub async fn service_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let method = req.method().to_string();
    let endpoint = req.uri().path().to_string();
    let ip_address = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let api_key = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let api_key = match api_key {
        Some(key) => key,
        None => {
            let mut log = AuditLog::new(
                "service_auth_failure".to_string(),
                None,
                endpoint,
                method,
                service_core::axum::http::StatusCode::UNAUTHORIZED.as_u16(),
                ip_address,
            );
            log.details = Some("Missing Authorization header".to_string());
            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = db.audit_logs().insert_one(log, None).await;
            });

            return Err(AppError::AuthError(anyhow::anyhow!("Missing or invalid Authorization header")));
        }
    };

    // 1. Identify key type and calculate lookup hash
    if !api_key.starts_with("svc_live_") && !api_key.starts_with("svc_test_") {
        return Err(AppError::AuthError(anyhow::anyhow!("Invalid API key format")));
    }

    let lookup_hash = ServiceAccount::calculate_lookup_hash(api_key);
    let cache_key = format!("svc_auth:{}", lookup_hash);

    // 2. Check Cache
    if let Ok(Some(cached_json)) = state.redis.get_cache(&cache_key).await {
        if let Ok(context) = serde_json::from_str::<ServiceContext>(&cached_json) {
            req.extensions_mut().insert(context);
            return Ok(next.run(req).await);
        }
    }

    // 3. Lookup in DB
    let account = state
        .db
        .service_accounts()
        .find_one(
            doc! {
                "$or": [
                    { "api_key_lookup_hash": &lookup_hash },
                    { "previous_api_key_lookup_hash": &lookup_hash }
                ]
            },
            None,
        )
        .await?;

    let account = match account {
        Some(acc) => acc,
        None => {
            let mut log = AuditLog::new(
                "service_auth_failure".to_string(),
                None,
                endpoint,
                method,
                service_core::axum::http::StatusCode::UNAUTHORIZED.as_u16(),
                ip_address,
            );
            log.details = Some("Invalid API key (no account found)".to_string());
            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = db.audit_logs().insert_one(log, None).await;
            });

            return Err(AppError::AuthError(anyhow::anyhow!("Invalid API key")));
        }
    };

    // 4. Check if enabled
    if !account.enabled {
        let mut log = AuditLog::new(
            "service_auth_failure".to_string(),
            Some(account.service_id.clone()),
            endpoint,
            method,
            service_core::axum::http::StatusCode::FORBIDDEN.as_u16(),
            ip_address,
        );
        log.service_name = Some(account.service_name.clone());
        log.details = Some("Service account disabled".to_string());
        let db = state.db.clone();
        tokio::spawn(async move {
            let _ = db.audit_logs().insert_one(log, None).await;
        });

        return Err(AppError::Forbidden(anyhow::anyhow!("Service account is disabled")));
    }

    // 5. Verify API Key with Argon2
    let verified = verify_password(
        &Password::new(api_key.to_string()),
        &PasswordHashString::new(account.api_key_hash.clone()),
    )
    .is_ok();

    if !verified {
        // Also check previous hash if exists
        let mut prev_verified = false;
        if let (Some(prev_hash), Some(prev_expiry)) =
            (&account.previous_api_key_hash, account.previous_key_expiry)
        {
            let now = chrono::Utc::now();
            if now < prev_expiry {
                prev_verified = verify_password(
                    &Password::new(api_key.to_string()),
                    &PasswordHashString::new(prev_hash.clone()),
                )
                .is_ok();
            }
        }

        if !prev_verified {
            let mut log = AuditLog::new(
                "service_auth_failure".to_string(),
                Some(account.service_id.clone()),
                endpoint,
                method,
                service_core::axum::http::StatusCode::UNAUTHORIZED.as_u16(),
                ip_address,
            );
            log.service_name = Some(account.service_name.clone());
            log.details = Some("Invalid API key (password verification failed)".to_string());
            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = db.audit_logs().insert_one(log, None).await;
            });

            return Err(AppError::AuthError(anyhow::anyhow!("Invalid API key")));
        }
    }

    // 6. Success - create context and cache it
    let context = ServiceContext {
        service_id: account.service_id.clone(),
        service_name: account.service_name.clone(),
        scopes: account.scopes.clone(),
    };

    // Audit log success
    let mut log = AuditLog::new(
        "service_auth_success".to_string(),
        Some(account.service_id.clone()),
        endpoint,
        method,
        service_core::axum::http::StatusCode::OK.as_u16(),
        ip_address,
    );
    log.service_name = Some(account.service_name.clone());
    log.scopes = Some(account.scopes.clone());
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        let _ = db_clone.audit_logs().insert_one(log, None).await;
    });

    // Update last_used_at (async, don't block)
    let db_clone = state.db.clone();
    let service_id_clone = account.service_id.clone();
    tokio::spawn(async move {
        let _ = db_clone
            .service_accounts()
            .update_one(
                doc! { "service_id": service_id_clone },
                doc! { "$set": { "last_used_at": mongodb::bson::DateTime::from_chrono(chrono::Utc::now()) } },
                None,
            )
            .await;
    });

    // Set cache (5-minute TTL)
    if let Ok(context_json) = serde_json::to_string(&context) {
        let _ = state.redis.set_cache(&cache_key, &context_json, 300).await;
    }

    req.extensions_mut().insert(context);
    Ok(next.run(req).await)
}

/// Extractor to easily get service context in handlers
pub struct ServiceAuth(pub ServiceContext);

#[async_trait]
impl<S> FromRequestParts<S> for ServiceAuth
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let context = parts.extensions.get::<ServiceContext>().ok_or_else(|| {
             AppError::InternalError(anyhow::anyhow!("Service context missing from request extensions"))
        })?;

        Ok(ServiceAuth(context.clone()))
    }
}
