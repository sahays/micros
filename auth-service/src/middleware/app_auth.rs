use mongodb::bson::doc;
use service_core::{
    axum::{
        async_trait,
        extract::{FromRequestParts, Request, State},
        http::{header, request::Parts},
        middleware::Next,
        response::Response,
    },
    error::AppError,
};

use crate::{services::AppTokenClaims, AppState};

/// Middleware to require app authentication
pub async fn app_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Extract token from X-App-Token or Authorization header
    let token = req
        .headers()
        .get("X-App-Token")
        .or_else(|| req.headers().get(header::AUTHORIZATION))
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            if value.starts_with("Bearer ") {
                value.strip_prefix("Bearer ").unwrap_or(value)
            } else {
                value
            }
        });

    let token = match token {
        Some(token) => token,
        None => {
            return Err(AppError::AuthError(anyhow::anyhow!(
                "Missing or invalid app token"
            )));
        }
    };

    // 2. Validate token
    let claims = match state.jwt.validate_app_token(token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err(AppError::AuthError(anyhow::anyhow!(
                "Invalid or expired app token"
            )));
        }
    };

    // 3. Check blacklist (for client revocation)
    let blacklist_key = format!("client:{}", claims.client_id);
    let is_revoked = state
        .redis
        .is_blacklisted(&blacklist_key)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to check client blacklist for {}: {}",
                claims.client_id,
                e
            );
            AppError::InternalError(anyhow::anyhow!("Failed to verify client status: {}", e))
        })?;

    if is_revoked {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Client has been revoked"
        )));
    }

    // 4. Check if client is still enabled in DB
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &claims.client_id }, None)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to query client {} from database: {}",
                claims.client_id,
                e
            );
            AppError::from(e)
        })?;

    let client = client.ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Client not found")))?;

    if !client.enabled {
        return Err(AppError::AuthError(anyhow::anyhow!("Client is disabled")));
    }

    // 5. Store claims in request extensions
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// Extractor to easily get app claims in handlers
pub struct CurrentApp(pub AppTokenClaims);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentApp
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<AppTokenClaims>().ok_or_else(|| {
            AppError::InternalError(anyhow::anyhow!(
                "App claims missing from request extensions"
            ))
        })?;

        Ok(CurrentApp(claims.clone()))
    }
}
