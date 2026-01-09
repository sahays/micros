use crate::AppState;
use service_core::axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use service_core::error::AppError;

pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Check for X-Admin-Api-Key header
    let api_key = headers
        .get("X-Admin-Api-Key")
        .and_then(|value| value.to_str().ok());

    match api_key {
        Some(key) if key == state.config.security.admin_api_key => {
            // Valid key, proceed
            Ok(next.run(request).await)
        }
        _ => {
            // Invalid or missing key
            tracing::warn!("Failed admin authentication attempt");
            Err(AppError::Unauthorized(anyhow::anyhow!(
                "Unauthorized: Invalid or missing admin API key"
            )))
        }
    }
}
