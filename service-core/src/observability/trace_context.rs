//! W3C Trace Context propagation for service-to-service calls.
//!
//! This module provides helpers to inject and extract W3C trace context headers
//! (traceparent and tracestate) for distributed tracing across microservices.
//!
//! See: https://www.w3.org/TR/trace-context/

use opentelemetry::trace::TraceContextExt;
use reqwest::header::HeaderMap;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Header name for W3C traceparent
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// Header name for W3C tracestate
pub const TRACESTATE_HEADER: &str = "tracestate";

/// Header name for request correlation ID
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Inject current trace context into HTTP request headers.
///
/// This extracts the current span's trace context and formats it as W3C
/// traceparent/tracestate headers for propagation to downstream services.
///
/// # Example
///
/// ```ignore
/// use service_core::observability::trace_context::inject_trace_context;
///
/// let mut headers = reqwest::header::HeaderMap::new();
/// inject_trace_context(&mut headers);
///
/// client.get(url)
///     .headers(headers)
///     .send()
///     .await?;
/// ```
pub fn inject_trace_context(headers: &mut HeaderMap) {
    let span = Span::current();
    let context = span.context();
    let otel_span = context.span();
    let span_context = otel_span.span_context();

    if span_context.is_valid() {
        // Format: version-trace_id-span_id-trace_flags
        // version is always "00" for the current spec
        let traceparent = format!(
            "00-{}-{}-{:02x}",
            span_context.trace_id(),
            span_context.span_id(),
            span_context.trace_flags().to_u8()
        );

        if let Ok(value) = traceparent.parse() {
            headers.insert(TRACEPARENT_HEADER, value);
        }

        // Include tracestate if present (check via header representation)
        let trace_state = span_context.trace_state();
        let tracestate_str = trace_state.header();
        if !tracestate_str.is_empty()
            && let Ok(value) = tracestate_str.parse()
        {
            headers.insert(TRACESTATE_HEADER, value);
        }
    }
}

/// Inject trace context and optional request ID into headers.
///
/// Convenience function that injects both trace context and request ID
/// for complete request correlation.
pub fn inject_trace_headers(headers: &mut HeaderMap, request_id: Option<&str>) {
    inject_trace_context(headers);

    if let Some(id) = request_id
        && let Ok(value) = id.parse()
    {
        headers.insert(REQUEST_ID_HEADER, value);
    }
}

/// Extract trace context from incoming request headers.
///
/// Returns the traceparent header value if present and valid.
pub fn extract_traceparent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract tracestate from incoming request headers.
pub fn extract_tracestate(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TRACESTATE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract request ID from incoming request headers.
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// A builder for creating HTTP clients with automatic trace context injection.
///
/// This wraps reqwest's RequestBuilder to automatically inject trace headers.
pub struct TracedRequest {
    request: reqwest::RequestBuilder,
}

impl TracedRequest {
    /// Create a new traced request from a reqwest RequestBuilder.
    pub fn new(request: reqwest::RequestBuilder) -> Self {
        Self { request }
    }

    /// Add a header to the request.
    pub fn header(self, key: &str, value: &str) -> Self {
        Self {
            request: self.request.header(key, value),
        }
    }

    /// Add JSON body to the request.
    pub fn json<T: serde::Serialize + ?Sized>(self, json: &T) -> Self {
        Self {
            request: self.request.json(json),
        }
    }

    /// Add bearer auth token.
    pub fn bearer_auth<T: std::fmt::Display>(self, token: T) -> Self {
        Self {
            request: self.request.bearer_auth(token),
        }
    }

    /// Send the request with trace context headers injected.
    pub async fn send(self) -> Result<reqwest::Response, reqwest::Error> {
        let mut headers = HeaderMap::new();
        inject_trace_context(&mut headers);

        self.request.headers(headers).send().await
    }

    /// Send the request with trace context and custom request ID.
    pub async fn send_with_request_id(
        self,
        request_id: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut headers = HeaderMap::new();
        inject_trace_headers(&mut headers, Some(request_id));

        self.request.headers(headers).send().await
    }
}

/// Extension trait for reqwest::Client to create traced requests.
pub trait TracedClientExt {
    fn traced_get(&self, url: &str) -> TracedRequest;
    fn traced_post(&self, url: &str) -> TracedRequest;
    fn traced_put(&self, url: &str) -> TracedRequest;
    fn traced_delete(&self, url: &str) -> TracedRequest;
}

impl TracedClientExt for reqwest::Client {
    fn traced_get(&self, url: &str) -> TracedRequest {
        TracedRequest::new(self.get(url))
    }

    fn traced_post(&self, url: &str) -> TracedRequest {
        TracedRequest::new(self.post(url))
    }

    fn traced_put(&self, url: &str) -> TracedRequest {
        TracedRequest::new(self.put(url))
    }

    fn traced_delete(&self, url: &str) -> TracedRequest {
        TracedRequest::new(self.delete(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_empty_context() {
        let mut headers = HeaderMap::new();
        inject_trace_context(&mut headers);
        // Without an active span, headers should be empty
        assert!(headers.is_empty());
    }

    #[test]
    fn test_extract_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert(
            TRACEPARENT_HEADER,
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
                .parse()
                .unwrap(),
        );

        let traceparent = extract_traceparent(&headers);
        assert_eq!(
            traceparent,
            Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string())
        );
    }

    #[test]
    fn test_extract_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID_HEADER, "abc-123".parse().unwrap());

        let request_id = extract_request_id(&headers);
        assert_eq!(request_id, Some("abc-123".to_string()));
    }
}
