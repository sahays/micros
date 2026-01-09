use service_core::axum::response::IntoResponse;

pub async fn metrics() -> impl IntoResponse {
    crate::services::metrics::get_metrics()
}
