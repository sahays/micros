//! gRPC interceptors for cross-cutting concerns.
//!
//! Provides interceptors for:
//! - Request ID extraction
//! - Tenant context extraction (app_id, tenant_id, user_id)
//! - Tenant/app ID injection for outgoing requests

use tonic::{Request, Status};
use uuid::Uuid;

/// gRPC metadata key for request ID.
pub const REQUEST_ID_KEY: &str = "x-request-id";

/// gRPC metadata key for tenant ID.
pub const TENANT_ID_KEY: &str = "x-tenant-id";

/// gRPC metadata key for app ID.
pub const APP_ID_KEY: &str = "x-app-id";

/// gRPC metadata key for user ID.
pub const USER_ID_KEY: &str = "x-user-id";

/// Tenant context extracted from BFF-provided headers.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// SaaS application owner (from x-app-id).
    pub app_id: String,
    /// Tenant within the app — org/club/school (from x-tenant-id).
    pub tenant_id: String,
    /// Individual user (from x-user-id).
    pub user_id: String,
}

/// Extract full tenant context from gRPC request metadata.
///
/// Requires x-app-id, x-tenant-id, and x-user-id headers.
/// Returns `Status::unauthenticated` if any are missing.
#[allow(clippy::result_large_err)]
pub fn extract_tenant_context<T>(request: &Request<T>) -> Result<TenantContext, Status> {
    let app_id = extract_app_id(request)
        .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;
    let tenant_id = extract_tenant_id(request)
        .ok_or_else(|| Status::unauthenticated("Missing x-tenant-id header"))?;
    let user_id = extract_user_id(request)
        .ok_or_else(|| Status::unauthenticated("Missing x-user-id header"))?;

    Ok(TenantContext {
        app_id,
        tenant_id,
        user_id,
    })
}

/// Extract app ID from x-app-id header and parse as UUID.
///
/// Used by PostgreSQL services that store tenant IDs as UUIDs.
#[allow(clippy::result_large_err)]
pub fn extract_app_id_uuid<T>(request: &Request<T>) -> Result<Uuid, Status> {
    let app_id = extract_app_id(request)
        .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;
    app_id
        .parse::<Uuid>()
        .map_err(|_| Status::invalid_argument("x-app-id is not a valid UUID"))
}

/// Extract tenant ID from incoming gRPC request metadata.
pub fn extract_tenant_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(TENANT_ID_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Inject tenant ID into outgoing gRPC request metadata.
pub fn inject_tenant_id<T>(request: &mut Request<T>, tenant_id: &str) {
    if let Ok(value) = tenant_id.parse() {
        request.metadata_mut().insert(TENANT_ID_KEY, value);
    }
}

/// Extract app ID from incoming gRPC request metadata.
pub fn extract_app_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(APP_ID_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Inject app ID into outgoing gRPC request metadata.
pub fn inject_app_id<T>(request: &mut Request<T>, app_id: &str) {
    if let Ok(value) = app_id.parse() {
        request.metadata_mut().insert(APP_ID_KEY, value);
    }
}

/// Extract user ID from incoming gRPC request metadata.
pub fn extract_user_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(USER_ID_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Inject user ID into outgoing gRPC request metadata.
pub fn inject_user_id<T>(request: &mut Request<T>, user_id: &str) {
    if let Ok(value) = user_id.parse() {
        request.metadata_mut().insert(USER_ID_KEY, value);
    }
}

/// Extract request ID from incoming gRPC request metadata.
pub fn extract_request_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(REQUEST_ID_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_and_extract_tenant_id() {
        let mut request = Request::new(());
        inject_tenant_id(&mut request, "tenant-123");

        let extracted = extract_tenant_id(&request);
        assert_eq!(extracted, Some("tenant-123".to_string()));
    }

    #[test]
    fn test_extract_request_id() {
        let mut request = Request::new(());
        if let Ok(value) = "req-456".parse() {
            request.metadata_mut().insert(REQUEST_ID_KEY, value);
        }

        let extracted = extract_request_id(&request);
        assert_eq!(extracted, Some("req-456".to_string()));
    }

    #[test]
    fn test_extract_tenant_context_success() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("x-app-id", "app-123".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", "tenant-456".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", "user-789".parse().unwrap());

        let ctx = extract_tenant_context(&request).unwrap();
        assert_eq!(ctx.app_id, "app-123");
        assert_eq!(ctx.tenant_id, "tenant-456");
        assert_eq!(ctx.user_id, "user-789");
    }

    #[test]
    fn test_extract_tenant_context_missing_app_id() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("x-tenant-id", "tenant-456".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-user-id", "user-789".parse().unwrap());

        let result = extract_tenant_context(&request);
        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("x-app-id"));
    }

    #[test]
    fn test_extract_app_id_uuid_success() {
        let mut request = Request::new(());
        let uuid = Uuid::new_v4();
        request
            .metadata_mut()
            .insert("x-app-id", uuid.to_string().parse().unwrap());

        let result = extract_app_id_uuid(&request).unwrap();
        assert_eq!(result, uuid);
    }

    #[test]
    fn test_extract_app_id_uuid_invalid() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("x-app-id", "not-a-uuid".parse().unwrap());

        let result = extract_app_id_uuid(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("x-user-id", "user-123".parse().unwrap());

        assert_eq!(extract_user_id(&request), Some("user-123".to_string()));
    }

    #[test]
    fn test_extract_user_id_missing() {
        let request: Request<()> = Request::new(());
        assert_eq!(extract_user_id(&request), None);
    }
}
