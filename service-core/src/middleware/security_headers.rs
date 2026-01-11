use axum::{extract::Request, http::header, middleware::Next, response::IntoResponse};

pub async fn security_headers_middleware(req: Request, next: Next) -> impl IntoResponse {
    let path = req.uri().path();
    let is_swagger_route = path.starts_with("/docs") || path == "/.well-known/openapi.json";

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_XSS_PROTECTION,
        header::HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Apply different CSP and framing policies for Swagger UI vs API routes
    if is_swagger_route {
        // Permissive CSP for Swagger UI: allow inline styles/scripts from same origin
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_static(
                "default-src 'self'; \
                 script-src 'self' 'unsafe-inline'; \
                 style-src 'self' 'unsafe-inline'; \
                 img-src 'self' data:; \
                 font-src 'self'; \
                 connect-src 'self'",
            ),
        );
        // Allow framing for Swagger UI
        headers.insert(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("SAMEORIGIN"),
        );
    } else {
        // Strict CSP for API routes: block everything except API responses
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        );
        headers.insert(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("DENY"),
        );
    }

    response
}
