//! gRPC trace context propagation interceptor.
//!
//! This module provides a tonic interceptor that extracts W3C trace context
//! (traceparent/tracestate) from incoming gRPC metadata and propagates it
//! to the current span.

use opentelemetry::propagation::TextMapPropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tonic::{Request, Status};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Header names for W3C trace context
#[allow(dead_code)]
const TRACEPARENT_HEADER: &str = "traceparent";
#[allow(dead_code)] // Reserved for future tracestate propagation
const TRACESTATE_HEADER: &str = "tracestate";

/// A text map extractor for gRPC metadata.
struct MetadataExtractor<'a>(&'a tonic::metadata::MetadataMap);

impl opentelemetry::propagation::Extractor for MetadataExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0
            .keys()
            .filter_map(|k| {
                if let tonic::metadata::KeyRef::Ascii(key) = k {
                    Some(key.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Extract trace context from gRPC metadata and set it as the parent context.
///
/// This function should be called at the beginning of each gRPC handler to
/// properly propagate distributed tracing context from upstream services.
///
/// # Example
///
/// ```ignore
/// async fn my_handler(&self, request: Request<MyRequest>) -> Result<Response<MyResponse>, Status> {
///     extract_trace_context(&request);
///     // ... handler logic
/// }
/// ```
pub fn extract_trace_context<T>(request: &Request<T>) {
    let metadata = request.metadata();
    let extractor = MetadataExtractor(metadata);
    let propagator = TraceContextPropagator::new();
    let context = propagator.extract(&extractor);

    // Set the extracted context as the parent of the current span
    Span::current().set_parent(context);
}

/// Log trace context information for debugging.
#[allow(dead_code)] // Available for debugging trace propagation
pub fn log_trace_context<T>(request: &Request<T>) {
    let metadata = request.metadata();

    if let Some(traceparent) = metadata.get(TRACEPARENT_HEADER) {
        if let Ok(value) = traceparent.to_str() {
            tracing::debug!(traceparent = %value, "Incoming trace context");
        }
    }

    if let Some(tracestate) = metadata.get(TRACESTATE_HEADER) {
        if let Ok(value) = tracestate.to_str() {
            tracing::debug!(tracestate = %value, "Incoming trace state");
        }
    }
}

/// A tonic interceptor that extracts and propagates trace context.
///
/// Add this interceptor to your gRPC service to automatically propagate
/// W3C trace context from incoming requests.
///
/// # Example
///
/// ```ignore
/// let service = BillingServiceServer::new(billing_service)
///     .intercept(trace_context_interceptor);
/// ```
#[allow(clippy::result_large_err)]
pub fn trace_context_interceptor(request: Request<()>) -> Result<Request<()>, Status> {
    let metadata = request.metadata();
    let extractor = MetadataExtractor(metadata);
    let propagator = TraceContextPropagator::new();
    let context = propagator.extract(&extractor);

    // Set the extracted context as the parent of the current span
    Span::current().set_parent(context);

    // Optionally log the trace context for debugging
    if tracing::enabled!(tracing::Level::DEBUG) {
        if let Some(traceparent) = request.metadata().get(TRACEPARENT_HEADER) {
            if let Ok(value) = traceparent.to_str() {
                tracing::debug!(traceparent = %value, "gRPC request with trace context");
            }
        }
    }

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::propagation::Extractor;
    use tonic::metadata::MetadataMap;

    #[test]
    fn test_metadata_extractor_get() {
        let mut metadata = MetadataMap::new();
        metadata.insert(
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
                .parse()
                .unwrap(),
        );

        let extractor = MetadataExtractor(&metadata);
        assert_eq!(
            extractor.get("traceparent"),
            Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
        );
    }

    #[test]
    fn test_metadata_extractor_missing_key() {
        let metadata = MetadataMap::new();
        let extractor = MetadataExtractor(&metadata);
        assert_eq!(extractor.get("traceparent"), None);
    }

    #[test]
    fn test_metadata_extractor_keys() {
        let mut metadata = MetadataMap::new();
        metadata.insert("traceparent", "value".parse().unwrap());
        metadata.insert("tracestate", "state".parse().unwrap());

        let extractor = MetadataExtractor(&metadata);
        let keys = extractor.keys();
        assert!(keys.contains(&"traceparent"));
        assert!(keys.contains(&"tracestate"));
    }
}
