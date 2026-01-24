//! Trace context interceptor for gRPC requests.
//!
//! Extracts W3C trace context from incoming gRPC metadata and sets up spans.

#![allow(clippy::result_large_err)]

use tonic::metadata::MetadataMap;
use tonic::{Request, Status};
use tracing::Span;

/// Keys for trace context propagation.
const TRACEPARENT_KEY: &str = "traceparent";
#[allow(dead_code)]
const TRACESTATE_KEY: &str = "tracestate";
const REQUEST_ID_KEY: &str = "x-request-id";

/// Metadata extractor for trace context.
pub struct MetadataExtractor<'a> {
    metadata: &'a MetadataMap,
}

impl<'a> MetadataExtractor<'a> {
    pub fn new(metadata: &'a MetadataMap) -> Self {
        Self { metadata }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|v| v.to_str().ok())
    }

    #[allow(dead_code)]
    pub fn keys(&self) -> Vec<&str> {
        self.metadata
            .keys()
            .filter_map(|k| match k {
                tonic::metadata::KeyRef::Ascii(k) => Some(k.as_str()),
                tonic::metadata::KeyRef::Binary(_) => None,
            })
            .collect()
    }
}

/// Extract trace context from gRPC metadata and create a span.
pub fn extract_trace_context<T>(request: &Request<T>) -> (Option<String>, Option<String>) {
    let metadata = request.metadata();
    let extractor = MetadataExtractor::new(metadata);

    let traceparent = extractor.get(TRACEPARENT_KEY).map(String::from);
    let request_id = extractor.get(REQUEST_ID_KEY).map(String::from);

    (traceparent, request_id)
}

/// Interceptor for extracting and propagating trace context.
pub fn trace_context_interceptor(request: Request<()>) -> Result<Request<()>, Status> {
    let (traceparent, request_id) = extract_trace_context(&request);

    // Log trace context for debugging
    if let Some(ref tp) = traceparent {
        tracing::debug!(traceparent = %tp, "Extracted traceparent from request");
    }

    if let Some(ref rid) = request_id {
        Span::current().record("request_id", rid.as_str());
    }

    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[test]
    fn test_metadata_extractor_get() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("x-request-id", "test-123".parse().unwrap());

        let extractor = MetadataExtractor::new(request.metadata());
        assert_eq!(extractor.get("x-request-id"), Some("test-123"));
        assert_eq!(extractor.get("nonexistent"), None);
    }

    #[test]
    fn test_metadata_extractor_keys() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("key-a", "value-a".parse().unwrap());
        request
            .metadata_mut()
            .insert("key-b", "value-b".parse().unwrap());

        let extractor = MetadataExtractor::new(request.metadata());
        let keys = extractor.keys();
        assert!(keys.contains(&"key-a"));
        assert!(keys.contains(&"key-b"));
    }

    #[test]
    fn test_metadata_extractor_missing_key() {
        let request: Request<()> = Request::new(());
        let extractor = MetadataExtractor::new(request.metadata());
        assert_eq!(extractor.get("missing"), None);
    }
}
