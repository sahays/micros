use crate::services::metrics::{HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION_SECONDS};
use service_core::axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    if let Some(counter) = HTTP_REQUESTS_TOTAL.get() {
        counter.with_label_values(&[&method, &path, &status]).inc();
    }

    if let Some(histogram) = HTTP_REQUEST_DURATION_SECONDS.get() {
        histogram
            .with_label_values(&[&method, &path, &status])
            .observe(duration);
    }

    response
}
