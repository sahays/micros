//! gRPC interceptors for cross-cutting concerns.
//!
//! Provides interceptors for:
//! - Request ID extraction
//! - Tenant ID injection/extraction

use tonic::Request;

/// gRPC metadata key for request ID.
pub const REQUEST_ID_KEY: &str = "x-request-id";

/// gRPC metadata key for tenant ID (used for metering).
pub const TENANT_ID_KEY: &str = "x-tenant-id";

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
}
