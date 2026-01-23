//! Shared capability checking infrastructure for BFF services.
//!
//! Provides optional capability checking for enhanced security across all services.
//! When enabled, validates bearer tokens and checks capabilities via auth-service.
//!
//! By default, services use a BFF trust model where the upstream service
//! (secure-frontend) handles authorization. This module provides an additional
//! security layer for direct access scenarios.

use super::{AuthClient, AuthClientConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::{Request, Status};

/// Pre-extracted metadata for capability checking.
/// Used to avoid borrowing the request across await points.
#[derive(Clone, Debug)]
pub struct CapabilityMetadata {
    pub token: String,
    pub org_node_id: Option<String>,
}

impl CapabilityMetadata {
    /// Extract metadata from a gRPC request.
    #[allow(clippy::result_large_err)]
    pub fn from_request<T>(request: &Request<T>) -> Result<Self, Status> {
        let token = extract_bearer_token(request)?;
        let org_node_id = extract_org_node_id(request);
        Ok(Self { token, org_node_id })
    }

    /// Try to extract metadata, returning None if auth header is missing.
    /// Used for endpoints that may not require auth (e.g., signed URLs).
    pub fn try_from_request<T>(request: &Request<T>) -> Option<Self> {
        let token = extract_bearer_token(request).ok()?;
        let org_node_id = extract_org_node_id(request);
        Some(Self { token, org_node_id })
    }
}

/// Authentication context returned after successful capability check.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: String,
}

/// Capability checker that delegates to auth-service.
///
/// When enabled, validates JWT tokens and checks capabilities via auth-service.
/// When disabled, uses BFF trust model (tenant context headers only).
#[derive(Clone)]
pub struct CapabilityChecker {
    inner: Option<Arc<RwLock<AuthClient>>>,
    enabled: bool,
}

impl CapabilityChecker {
    /// Create a new capability checker.
    ///
    /// If `auth_endpoint` is provided, capability checking is enabled.
    /// Otherwise, the BFF trust model is used.
    pub async fn new(auth_endpoint: Option<&str>) -> Result<Self, tonic::transport::Error> {
        match auth_endpoint {
            Some(endpoint) if !endpoint.is_empty() => {
                let client = AuthClient::new(AuthClientConfig {
                    endpoint: endpoint.to_string(),
                    connect_timeout: Duration::from_secs(5),
                    request_timeout: Duration::from_secs(10),
                })
                .await?;

                tracing::info!(
                    auth_endpoint = endpoint,
                    "Capability enforcement enabled via auth-service"
                );

                Ok(Self {
                    inner: Some(Arc::new(RwLock::new(client))),
                    enabled: true,
                })
            }
            _ => {
                tracing::info!("Capability enforcement disabled (BFF trust model)");
                Ok(Self {
                    inner: None,
                    enabled: false,
                })
            }
        }
    }

    /// Create a disabled checker (BFF trust model).
    pub fn disabled() -> Self {
        Self {
            inner: None,
            enabled: false,
        }
    }

    /// Check if capability enforcement is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Require capability for the given request.
    ///
    /// If capability checking is enabled:
    /// - Extracts bearer token from authorization header
    /// - Validates token via auth-service
    /// - Checks if user has the required capability
    ///
    /// If capability checking is disabled:
    /// - Returns Ok with default AuthContext (trusts BFF)
    ///
    /// # Arguments
    /// * `request` - The gRPC request with metadata
    /// * `capability` - The required capability key (e.g., "document:upload")
    ///
    /// # Returns
    /// * `Ok(AuthContext)` - Capability granted or checking disabled
    /// * `Err(Status)` - Token invalid or capability missing
    #[allow(clippy::result_large_err)]
    pub async fn require_capability<T>(
        &self,
        request: &Request<T>,
        capability: &str,
    ) -> Result<AuthContext, Status> {
        // Skip if capability checking is disabled (BFF trust model)
        if !self.enabled {
            // Extract user/tenant from headers for BFF trust model
            return Ok(extract_auth_context_from_headers(request));
        }

        // Extract needed values before any await
        let metadata = CapabilityMetadata::from_request(request)?;
        self.require_capability_from_metadata(&metadata, capability)
            .await
    }

    /// Require capability using pre-extracted metadata.
    /// Use this for streaming requests where the request can't be borrowed across await points.
    #[allow(clippy::result_large_err)]
    pub async fn require_capability_from_metadata(
        &self,
        metadata: &CapabilityMetadata,
        capability: &str,
    ) -> Result<AuthContext, Status> {
        let client = match &self.inner {
            Some(c) => c,
            None => {
                // BFF trust model - capability checking disabled
                return Ok(AuthContext {
                    user_id: "system".to_string(),
                    tenant_id: String::new(),
                });
            }
        };

        // Validate token via auth-service
        let mut auth_client = client.write().await;
        let validate_response = auth_client
            .validate_token(metadata.token.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to validate token: {}", e)))?;

        if !validate_response.valid {
            return Err(Status::unauthenticated("Invalid or expired token"));
        }

        let claims = validate_response
            .claims
            .ok_or_else(|| Status::internal("Token valid but missing claims"))?;

        // Check capability via auth-service
        let check_response = auth_client
            .check_capability(
                &claims.sub,                                      // user_id
                &claims.app_id,                                   // tenant_id
                metadata.org_node_id.clone().unwrap_or_default(), // org_node_id
                capability.to_string(),                           // capability
            )
            .await
            .map_err(|e| {
                tracing::warn!(
                    user_id = %claims.sub,
                    capability = capability,
                    error = %e,
                    "Capability check failed"
                );
                Status::internal(format!("Failed to check capability: {}", e))
            })?;

        if !check_response.allowed {
            tracing::warn!(
                user_id = %claims.sub,
                capability = capability,
                "Permission denied: missing capability"
            );
            return Err(Status::permission_denied(format!(
                "Missing capability: {}",
                capability
            )));
        }

        Ok(AuthContext {
            user_id: claims.sub,
            tenant_id: claims.app_id,
        })
    }

    /// Require authentication only (no capability check).
    ///
    /// If capability checking is enabled:
    /// - Extracts bearer token from authorization header
    /// - Validates token via auth-service
    ///
    /// If capability checking is disabled:
    /// - Returns Ok (trusts BFF)
    #[allow(clippy::result_large_err)]
    pub async fn require_auth<T>(&self, request: &Request<T>) -> Result<AuthContext, Status> {
        // Skip if capability checking is disabled (BFF trust model)
        if !self.enabled {
            return Ok(extract_auth_context_from_headers(request));
        }

        let metadata = CapabilityMetadata::from_request(request)?;
        self.require_auth_from_metadata(&metadata).await
    }

    /// Require authentication using pre-extracted metadata.
    /// Use this for streaming requests where the request can't be borrowed across await points.
    #[allow(clippy::result_large_err)]
    pub async fn require_auth_from_metadata(
        &self,
        metadata: &CapabilityMetadata,
    ) -> Result<AuthContext, Status> {
        let client = match &self.inner {
            Some(c) => c,
            None => {
                // BFF trust model - auth checking disabled
                return Ok(AuthContext {
                    user_id: "system".to_string(),
                    tenant_id: String::new(),
                });
            }
        };

        let mut auth_client = client.write().await;
        let validate_response = auth_client
            .validate_token(metadata.token.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to validate token: {}", e)))?;

        if !validate_response.valid {
            return Err(Status::unauthenticated("Invalid or expired token"));
        }

        let claims = validate_response
            .claims
            .ok_or_else(|| Status::internal("Token valid but missing claims"))?;

        Ok(AuthContext {
            user_id: claims.sub,
            tenant_id: claims.app_id,
        })
    }
}

/// Extract bearer token from gRPC request metadata.
#[allow(clippy::result_large_err)]
pub fn extract_bearer_token<T>(request: &Request<T>) -> Result<String, Status> {
    request
        .metadata()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("Invalid authorization header encoding"))?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| Status::unauthenticated("Invalid Bearer token format"))
}

/// Extract org_node_id from gRPC request metadata.
pub fn extract_org_node_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract auth context from BFF trust headers.
fn extract_auth_context_from_headers<T>(request: &Request<T>) -> AuthContext {
    let user_id = request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("system")
        .to_string();

    let tenant_id = request
        .metadata()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    AuthContext { user_id, tenant_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disabled_checker_allows_all() {
        let checker = CapabilityChecker::disabled();
        assert!(!checker.is_enabled());

        let request: Request<()> = Request::new(());
        let result = checker
            .require_capability(&request, "some:capability")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disabled_checker_returns_auth_context() {
        let checker = CapabilityChecker::disabled();

        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-user-id", "user-123".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", "tenant-456".parse().unwrap());

        let result = checker
            .require_capability(&request, "some:capability")
            .await;
        assert!(result.is_ok());

        let auth_context = result.unwrap();
        assert_eq!(auth_context.user_id, "user-123");
        assert_eq!(auth_context.tenant_id, "tenant-456");
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let request: Request<()> = Request::new(());
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message()
                .contains("Missing authorization header")
        );
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Basic abc123".parse().unwrap());

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message()
                .contains("Invalid Bearer token format")
        );
    }

    #[test]
    fn test_extract_bearer_token_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token-123".parse().unwrap());

        let result = extract_bearer_token(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-token-123");
    }

    #[test]
    fn test_extract_org_node_id_missing() {
        let request: Request<()> = Request::new(());
        let result = extract_org_node_id(&request);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_org_node_id_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-org-id", "org-789".parse().unwrap());

        let result = extract_org_node_id(&request);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "org-789");
    }

    #[test]
    fn test_capability_metadata_from_request() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", "org-123".parse().unwrap());

        let metadata = CapabilityMetadata::from_request(&request);
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.token, "test-token");
        assert_eq!(metadata.org_node_id, Some("org-123".to_string()));
    }

    #[test]
    fn test_capability_metadata_try_from_request() {
        // Without auth header - should return None
        let request: Request<()> = Request::new(());
        let metadata = CapabilityMetadata::try_from_request(&request);
        assert!(metadata.is_none());

        // With auth header - should return Some
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token".parse().unwrap());

        let metadata = CapabilityMetadata::try_from_request(&request);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().token, "test-token");
    }
}
