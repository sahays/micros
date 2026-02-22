//! Authentication context extraction for auth-service.
//!
//! Extracts identity from BFF-provided headers.

use tonic::{Request, Status};
use uuid::Uuid;

/// Authentication context extracted from request headers.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// User making the request (from x-user-id).
    pub user_id: Uuid,
    /// Application identity (from x-app-id).
    pub app_id: Uuid,
}

/// Extract auth context from gRPC request headers.
///
/// Reads x-app-id and x-user-id, parsing both as UUIDs.
#[allow(clippy::result_large_err)]
pub fn extract_auth_context<T>(request: &Request<T>) -> Result<AuthContext, Status> {
    let app_id_str = request
        .metadata()
        .get("x-app-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;

    let app_id = app_id_str
        .parse::<Uuid>()
        .map_err(|_| Status::invalid_argument("x-app-id is not a valid UUID"))?;

    let user_id_str = request
        .metadata()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated("Missing x-user-id header"))?;

    let user_id = user_id_str
        .parse::<Uuid>()
        .map_err(|_| Status::invalid_argument("x-user-id is not a valid UUID"))?;

    Ok(AuthContext { user_id, app_id })
}
