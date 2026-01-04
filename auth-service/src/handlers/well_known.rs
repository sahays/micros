use crate::AppState;
use axum::{extract::State, http::header, response::IntoResponse, Json};

/// Get JSON Web Key Set (JWKS)
#[utoipa::path(
    get,
    path = "/.well-known/jwks.json",
    responses(
        (status = 200, description = "Public JWKS returned")
    ),
    tag = "Well-Known"
)]
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
