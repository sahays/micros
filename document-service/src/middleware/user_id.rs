use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use service_core::error::AppError;

/// UserId extractor for document-service
///
/// Extracts user_id from X-User-ID header sent by trusted service clients (secure-frontend)
/// via HMAC-signed requests. This enables:
/// 1. User context propagation across services
/// 2. Document ownership tracking (owner_id)
/// 3. User-scoped distributed tracing
///
/// Security: X-User-ID is only trusted when the request signature is valid.
/// The signature middleware must run BEFORE this extractor to validate the request
/// came from a trusted service (secure-frontend).
#[derive(Debug, Clone)]
pub struct UserId(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user_id = parts
            .headers
            .get("X-User-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::AuthError(anyhow::anyhow!(
                    "Missing X-User-ID header (required from BFF)"
                ))
            })?;

        // Add to tracing span for observability
        tracing::Span::current().record("user_id", user_id);

        Ok(UserId(user_id.to_string()))
    }
}
