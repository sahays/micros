use crate::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};

pub async fn jwks(State(state): State<AppState>) -> impl IntoResponse {
    match state.jwt.get_jwks() {
        Ok(jwks) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            Json(jwks),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate JWKS");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
            )
                .into_response()
        }
    }
}
