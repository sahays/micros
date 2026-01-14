use axum::{response::IntoResponse, Json, http::StatusCode};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok", "service": "payment-service" })))
}

pub async fn metrics() -> String {
    crate::services::metrics::get_metrics()
}
