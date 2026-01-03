use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::{
    services::{AccessTokenClaims, JwtService},
    AppState,
};

/// Middleware to require authentication
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let token = match token {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing or invalid Authorization header".to_string(),
                }),
            ));
        }
    };

    let claims = match state.jwt.validate_access_token(token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid or expired token".to_string(),
                }),
            ));
        }
    };

    // Store claims in request extensions so handlers can access them
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Extractor to easily get claims in handlers
pub struct AuthUser(pub AccessTokenClaims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<AccessTokenClaims>()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Auth claims missing from request extensions".to_string(),
                }),
            ))?;

        Ok(AuthUser(claims.clone()))
    }
}
