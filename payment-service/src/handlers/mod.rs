pub mod razorpay;
pub mod transactions;
pub mod upi;

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "service": "payment-service" })),
    )
}

pub async fn metrics() -> String {
    crate::services::metrics::get_metrics()
}
