//! Tenant context middleware for multi-tenancy support.
//!
//! This middleware extracts tenant information (app_id, org_id) from:
//! 1. JWT access token claims (for authenticated user requests)
//! 2. App token claims (for service-to-service requests)
//! 3. Request headers (for admin operations)

use service_core::{
    axum::{
        async_trait,
        extract::{FromRequestParts, Request},
        http::request::Parts,
        middleware::Next,
        response::Response,
    },
    error::AppError,
};

use crate::services::AccessTokenClaims;

/// Tenant context extracted from the request.
/// Available in handlers via State or Extension.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Application ID (maps to Client.client_id)
    pub app_id: String,
    /// Organization ID (optional for app-level operations)
    pub org_id: Option<String>,
}

impl TenantContext {
    /// Create a new tenant context with both app and org.
    pub fn new(app_id: String, org_id: String) -> Self {
        Self {
            app_id,
            org_id: Some(org_id),
        }
    }

    /// Create a tenant context with only app (for app-level operations).
    pub fn app_only(app_id: String) -> Self {
        Self {
            app_id,
            org_id: None,
        }
    }

    /// Get org_id or return an error if not set.
    pub fn require_org_id(&self) -> Result<&str, TenantError> {
        self.org_id.as_deref().ok_or(TenantError::OrgIdRequired)
    }
}

/// Errors related to tenant context extraction.
#[derive(Debug, Clone)]
pub enum TenantError {
    /// No tenant context found in request.
    Missing,
    /// Organization ID is required but not provided.
    OrgIdRequired,
    /// Invalid tenant context.
    Invalid(String),
}

impl std::fmt::Display for TenantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantError::Missing => write!(f, "Tenant context not found"),
            TenantError::OrgIdRequired => write!(f, "Organization ID is required"),
            TenantError::Invalid(msg) => write!(f, "Invalid tenant context: {}", msg),
        }
    }
}

impl std::error::Error for TenantError {}

/// Middleware to extract tenant context from authenticated requests.
///
/// This should be applied after authentication middleware.
/// It reads the AccessTokenClaims from request extensions and
/// creates a TenantContext for downstream handlers.
pub async fn tenant_context_middleware(request: Request, next: Next) -> Response {
    // Try to extract tenant context from JWT claims in extensions
    let tenant_context = request
        .extensions()
        .get::<AccessTokenClaims>()
        .map(|claims| TenantContext::new(claims.app_id.clone(), claims.org_id.clone()));

    // If we have tenant context, add it to extensions
    let mut request = request;
    if let Some(ctx) = tenant_context {
        request.extensions_mut().insert(ctx);
    }

    next.run(request).await
}

/// Extractor for TenantContext from request extensions.
///
/// Use this in handlers that require tenant context:
/// ```ignore
/// async fn handler(tenant: TenantContext) -> impl IntoResponse {
///     // Use tenant.app_id and tenant.org_id
/// }
/// ```
#[async_trait]
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Tenant context not found")))
    }
}
