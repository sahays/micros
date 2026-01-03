use axum::{extract::State, http::header, response::IntoResponse, Json};
use crate::AppState;

pub async fn jwks(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.jwt.get_jwks() {
        Ok(jwks) => (
            [
                (header::CACHE_CONTROL, "public, max-age=3600"),
                (header::CONTENT_TYPE, "application/json"),
            ],
            Json(jwks),
        ).into_response(),
        Err(e) => {
            tracing::error!("Failed to generate JWKS: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            ).into_response()
        }
    }
}
