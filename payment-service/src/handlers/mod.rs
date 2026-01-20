//! HTTP handlers for payment-service.
//!
//! This module only contains health check endpoints.
//! All business logic is handled via gRPC.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "service": "payment-service" })),
    )
}

pub async fn readiness_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ready" })))
}
