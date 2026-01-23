//! Capability enforcement for notification-service gRPC endpoints.
//!
//! Provides optional capability checking for enhanced security.
//! When enabled, validates bearer tokens and checks capabilities via auth-service.
//!
//! By default, notification-service uses a BFF trust model where the upstream
//! service (secure-frontend) handles authorization. This module provides
//! an additional security layer for direct access scenarios.

use service_core::grpc::{AuthClient, AuthClientConfig};
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
    pub fn try_from_request<T>(request: &Request<T>) -> Option<Self> {
        let token = extract_bearer_token(request).ok()?;
        let org_node_id = extract_org_node_id(request);
        Some(Self { token, org_node_id })
    }
}

/// Capability checker for Notification operations.
///
/// When enabled, validates JWT tokens and checks capabilities via auth-service.
/// When disabled, uses BFF trust model (request metadata only).
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
    /// - Returns Ok (trusts BFF)
    #[allow(clippy::result_large_err)]
    pub async fn require_capability<T>(
        &self,
        request: &Request<T>,
        capability: &str,
    ) -> Result<(), Status> {
        // Skip if capability checking is disabled (BFF trust model)
        if !self.enabled {
            return Ok(());
        }

        // Extract needed values before any await
        let metadata = CapabilityMetadata::from_request(request)?;
        self.require_capability_from_metadata(&metadata, capability)
            .await
    }

    /// Require capability using pre-extracted metadata.
    #[allow(clippy::result_large_err)]
    pub async fn require_capability_from_metadata(
        &self,
        metadata: &CapabilityMetadata,
        capability: &str,
    ) -> Result<(), Status> {
        let client = match &self.inner {
            Some(c) => c,
            None => return Ok(()), // BFF trust model - capability checking disabled
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

        Ok(())
    }

    /// Require authentication only (no capability check).
    #[allow(clippy::result_large_err)]
    pub async fn require_auth<T>(&self, request: &Request<T>) -> Result<(), Status> {
        // Skip if capability checking is disabled (BFF trust model)
        if !self.enabled {
            return Ok(());
        }

        let metadata = CapabilityMetadata::from_request(request)?;
        self.require_auth_from_metadata(&metadata).await
    }

    /// Require authentication using pre-extracted metadata.
    #[allow(clippy::result_large_err)]
    pub async fn require_auth_from_metadata(
        &self,
        metadata: &CapabilityMetadata,
    ) -> Result<(), Status> {
        let client = match &self.inner {
            Some(c) => c,
            None => return Ok(()), // BFF trust model - auth checking disabled
        };

        let mut auth_client = client.write().await;
        let validate_response = auth_client
            .validate_token(metadata.token.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to validate token: {}", e)))?;

        if !validate_response.valid {
            return Err(Status::unauthenticated("Invalid or expired token"));
        }

        Ok(())
    }
}

/// Extract bearer token from gRPC request metadata.
#[allow(clippy::result_large_err)]
fn extract_bearer_token<T>(request: &Request<T>) -> Result<String, Status> {
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
fn extract_org_node_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Notification service capabilities.
pub mod capabilities {
    /// Send email notifications.
    pub const NOTIFICATION_EMAIL_SEND: &str = "notification.email:send";

    /// Send SMS notifications.
    pub const NOTIFICATION_SMS_SEND: &str = "notification.sms:send";

    /// Send push notifications.
    pub const NOTIFICATION_PUSH_SEND: &str = "notification.push:send";

    /// Send batch notifications.
    pub const NOTIFICATION_BATCH_SEND: &str = "notification.batch:send";

    /// View notifications.
    pub const NOTIFICATION_READ: &str = "notification:read";
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
            .require_capability(&request, "notification.email:send")
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let request: Request<()> = Request::new(());
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Missing authorization header"));
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Basic abc123".parse().unwrap());

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Invalid Bearer token format"));
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
}
