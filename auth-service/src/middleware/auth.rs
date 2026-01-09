use service_core::{axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::Next,
    response::Response,
    async_trait,
}, error::AppError};

use crate::{services::AccessTokenClaims, AppState};

/// Middleware to require authentication
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let token = match token {
        Some(token) => token,
        None => {
            return Err(AppError::AuthError(anyhow::anyhow!("Missing or invalid Authorization header")));
        }
    };

    let claims = match state.jwt.validate_access_token(token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err(AppError::AuthError(anyhow::anyhow!("Invalid or expired token")));
        }
    };

    // Check blacklist
    let is_blacklisted = state.redis.is_blacklisted(&claims.jti).await?;

    if is_blacklisted {
        return Err(AppError::AuthError(anyhow::anyhow!("Token has been revoked")));
    }

    // Store claims in request extensions so handlers can access them
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

/// Extractor to easily get claims in handlers
pub struct AuthUser(pub AccessTokenClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<AccessTokenClaims>().ok_or_else(|| {
             AppError::InternalError(anyhow::anyhow!("Auth claims missing from request extensions"))
        })?;

        Ok(AuthUser(claims.clone()))
    }
}
