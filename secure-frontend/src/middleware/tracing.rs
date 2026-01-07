use axum::http::HeaderValue;
use axum::{extract::Request, middleware::Next, response::Response};
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

    let mut response = next.run(req).await;

    // Add request_id to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    response
}
