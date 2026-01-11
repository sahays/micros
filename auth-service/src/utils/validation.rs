use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::dtos::ErrorResponse;

pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            let err_msg = format!("Json parse error: {}", e);
            tracing::warn!(error = %e, "Request JSON parse failed");
            let err_resp = ErrorResponse { error: err_msg };
            (StatusCode::BAD_REQUEST, Json(err_resp)).into_response()
        })?;

        value.validate().map_err(|e| {
            let err_msg = format!("Validation error: {}", e);
            tracing::warn!(error = %e, "Request validation failed");
            let err_resp = ErrorResponse { error: err_msg };
            (StatusCode::UNPROCESSABLE_ENTITY, Json(err_resp)).into_response()
        })?;

        Ok(ValidatedJson(value))
    }
}
