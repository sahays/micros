use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Json,
};
use mongodb::bson::doc;
use serde::Serialize;

use crate::{services::AppTokenClaims, AppState};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Middleware to require app authentication
pub async fn app_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Extract token from X-App-Token or Authorization header
    let token = req
        .headers()
        .get("X-App-Token")
        .or_else(|| req.headers().get(header::AUTHORIZATION))
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            if value.starts_with("Bearer ") {
                value.strip_prefix("Bearer ").unwrap()
            } else {
                value
            }
        });

    let token = match token {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing or invalid app token".to_string(),
                }),
            ));
        }
    };

    // 2. Validate token
    let claims = match state.jwt.validate_app_token(token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid or expired app token".to_string(),
                }),
            ));
        }
    };

    // 3. Check blacklist (for client revocation)
    let blacklist_key = format!("client:{}", claims.client_id);
    let is_revoked = state
        .redis
        .is_blacklisted(&blacklist_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Redis error checking client revocation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if is_revoked {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Client has been revoked".to_string(),
            }),
        ));
    }

    // 4. Check if client is still enabled in DB
    // Optimization: Consider caching this check in Redis if high traffic
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &claims.client_id }, None)
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
                error: "Client not found".to_string(),
            }),
        )
    })?;

    if !client.enabled {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Client is disabled".to_string(),
            }),
        ));
    }

    // 5. Store claims in request extensions
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// Extractor to easily get app claims in handlers
pub struct CurrentApp(pub AppTokenClaims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for CurrentApp
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<AppTokenClaims>().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "App claims missing from request extensions".to_string(),
            }),
        ))?;

        Ok(CurrentApp(claims.clone()))
    }
}
