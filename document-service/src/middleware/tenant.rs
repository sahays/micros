//! Tenant context middleware for multi-tenancy support.
//!
//! Extracts tenant information (app_id, org_id, user_id) from request headers.
//! These headers are set by the BFF (secure-frontend) after authenticating the user
//! and validating their tenant membership.
//!
//! Security: Headers are only trusted when the request signature is valid.
//! The signature middleware must run BEFORE this extractor.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use service_core::error::AppError;

/// Tenant context extracted from request headers.
///
/// Contains all tenant information needed for multi-tenant document operations.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Application ID (maps to registered client in auth-service)
    pub app_id: String,
    /// Organization ID within the application
    pub org_id: String,
    /// User ID who is making the request
    pub user_id: String,
}

impl TenantContext {
    /// Create a new tenant context.
    pub fn new(app_id: String, org_id: String, user_id: String) -> Self {
        Self {
            app_id,
            org_id,
            user_id,
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract app_id from X-App-ID header
        let app_id = parts
            .headers
            .get("X-App-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::AuthError(anyhow::anyhow!(
                    "Missing X-App-ID header (required from BFF)"
                ))
            })?;

        // Extract org_id from X-Org-ID header
        let org_id = parts
            .headers
            .get("X-Org-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::AuthError(anyhow::anyhow!(
                    "Missing X-Org-ID header (required from BFF)"
                ))
            })?;

        // Extract user_id from X-User-ID header
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
        let span = tracing::Span::current();
        span.record("app_id", app_id);
        span.record("org_id", org_id);
        span.record("user_id", user_id);

        Ok(TenantContext::new(
            app_id.to_string(),
            org_id.to_string(),
            user_id.to_string(),
        ))
    }
}
