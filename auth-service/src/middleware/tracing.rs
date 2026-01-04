use axum::http::HeaderValue;
use axum::{extract::Request, middleware::Next, response::Response};
use tracing::info_span;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // UUID strings should always be valid HeaderValue, but handle error gracefully
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    // Create a span for the request that includes the request_id
    let span = info_span!(
        "http_request",
        request_id = %request_id,
        method = %req.method(),
        uri = %req.uri(),
    );

    let mut response = {
        let _guard = span.enter();
        next.run(req).await
    };

    // Add request_id to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    response
}
