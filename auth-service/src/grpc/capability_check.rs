//! Capability enforcement for gRPC endpoints.
//!
//! Provides helper functions to extract auth context from gRPC requests
//! and verify that the caller has the required capability.

use crate::AppState;
use tonic::{Request, Status};
use uuid::Uuid;

/// Authentication context extracted from a valid access token.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The authenticated user's ID.
    pub user_id: Uuid,
    /// The tenant ID from the token (app_id claim).
    pub tenant_id: Uuid,
}

/// Check if caller has the required capability.
///
/// Extracts the bearer token from the request, validates it, and checks
/// if the user has the required capability through any of their role assignments.
///
/// The `*` capability (superadmin) grants access to all endpoints.
///
/// # Arguments
/// * `state` - Application state containing JWT service and database
/// * `request` - The gRPC request containing authorization metadata
/// * `required_capability` - The capability key required for this operation
///
/// # Returns
/// * `Ok(AuthContext)` - The user is authenticated and has the required capability
/// * `Err(Status)` - Authentication or authorization failed
pub async fn require_capability<T: std::fmt::Debug>(
    state: &AppState,
    request: &Request<T>,
    required_capability: &str,
) -> Result<AuthContext, Status> {
    let token = extract_bearer_token(request)?;

    let claims = state
        .jwt
        .validate_access_token(&token)
        .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| Status::internal("Invalid user_id in token"))?;
    let tenant_id = Uuid::parse_str(&claims.app_id)
        .map_err(|_| Status::internal("Invalid tenant_id in token"))?;

    // Get user's active assignments
    let assignments = state
        .db
        .find_active_assignments_for_user(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch user assignments");
            Status::internal("Database error")
        })?;

    // Check if any assignment grants the required capability
    let mut has_capability = false;
    for assignment in assignments {
        let caps = state
            .db
            .get_role_capabilities(assignment.role_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch role capabilities");
                Status::internal("Database error")
            })?;

        // Check for exact match or superadmin wildcard
        if caps.contains(&"*".to_string()) || caps.contains(&required_capability.to_string()) {
            has_capability = true;
            break;
        }
    }

    if !has_capability {
        tracing::warn!(
            user_id = %user_id,
            required_capability = %required_capability,
            "Permission denied: missing capability"
        );
        return Err(Status::permission_denied(format!(
            "Missing capability: {}",
            required_capability
        )));
    }

    Ok(AuthContext { user_id, tenant_id })
}

/// Extract the authenticated user context without checking capabilities.
///
/// Use this for self-service endpoints where the user only needs to be authenticated.
///
/// # Arguments
/// * `state` - Application state containing JWT service
/// * `request` - The gRPC request containing authorization metadata
///
/// # Returns
/// * `Ok(AuthContext)` - The user is authenticated
/// * `Err(Status)` - Authentication failed
pub async fn require_auth<T: std::fmt::Debug>(
    state: &AppState,
    request: &Request<T>,
) -> Result<AuthContext, Status> {
    let token = extract_bearer_token(request)?;

    let claims = state
        .jwt
        .validate_access_token(&token)
        .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| Status::internal("Invalid user_id in token"))?;
    let tenant_id = Uuid::parse_str(&claims.app_id)
        .map_err(|_| Status::internal("Invalid tenant_id in token"))?;

    Ok(AuthContext { user_id, tenant_id })
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

/// Validate admin API key from request metadata.
///
/// Used for administrative operations that require the X-Admin-Api-Key header.
#[allow(clippy::result_large_err)]
pub fn require_admin_api_key<T>(
    config: &crate::config::AuthConfig,
    request: &Request<T>,
) -> Result<(), Status> {
    let provided_key = request
        .metadata()
        .get("x-admin-api-key")
        .ok_or_else(|| Status::unauthenticated("Missing X-Admin-Api-Key header"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("Invalid X-Admin-Api-Key header encoding"))?;

    if provided_key != config.security.admin_api_key {
        return Err(Status::unauthenticated("Invalid admin API key"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token_missing_header() {
        let request: Request<()> = Request::new(());
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Missing authorization header"));
    }
}
