//! Tenant context middleware for multi-tenancy support.
//!
//! Extracts tenant information (app_id, tenant_id, user_id) from request headers.
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
    /// Tenant ID within the application (org/club/school)
    pub tenant_id: String,
    /// User ID who is making the request
    pub user_id: String,
}

impl TenantContext {
    /// Create a new tenant context.
    pub fn new(app_id: String, tenant_id: String, user_id: String) -> Self {
        Self {
            app_id,
            tenant_id,
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

        // Extract tenant_id from X-Tenant-ID header
        let tenant_id = parts
            .headers
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::AuthError(anyhow::anyhow!(
                    "Missing X-Tenant-ID header (required from BFF)"
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
        span.record("tenant_id", tenant_id);
        span.record("user_id", user_id);

        Ok(TenantContext::new(
            app_id.to_string(),
            tenant_id.to_string(),
            user_id.to_string(),
        ))
    }
}
