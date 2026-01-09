use crate::AppState;
use service_core::{
    axum::{extract::State, http::header, response::IntoResponse, Json},
    error::AppError,
};

/// Get JSON Web Key Set (JWKS)
#[utoipa::path(
    get,
    path = "/.well-known/jwks.json",
    responses(
        (status = 200, description = "Public JWKS returned")
    ),
    tag = "Well-Known"
)]
pub async fn jwks(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let jwks = state.jwt.get_jwks()?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        Json(jwks),
    ))
}
