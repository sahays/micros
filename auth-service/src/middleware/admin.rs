use crate::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Check for X-Admin-Api-Key header
    let api_key = headers
        .get("X-Admin-Api-Key")
        .and_then(|value| value.to_str().ok());

    match api_key {
        Some(key) if key == state.config.security.admin_api_key => {
            // Valid key, proceed
            next.run(request).await
        }
        _ => {
            // Invalid or missing key
            tracing::warn!("Failed admin authentication attempt");
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized: Invalid or missing admin API key" })),
            )
                .into_response()
        }
    }
}
