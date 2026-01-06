use prometheus::{Encoder, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder};
use std::sync::OnceLock;

// Global registry
pub static REGISTRY: OnceLock<Registry> = OnceLock::new();

// Metrics
pub static HTTP_REQUESTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static HTTP_REQUEST_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();

pub fn init_metrics() {
    let registry = Registry::new();

    let requests_total = IntCounterVec::new(
        Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("metric can be created");

    let request_duration = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        ),
        &["method", "path", "status"],
    )
    .expect("metric can be created");

    registry
        .register(Box::new(requests_total.clone()))
        .expect("collector can be registered");
    registry
        .register(Box::new(request_duration.clone()))
        .expect("collector can be registered");

    // Initialize globals
    let _ = REGISTRY.set(registry);
    let _ = HTTP_REQUESTS_TOTAL.set(requests_total);
    let _ = HTTP_REQUEST_DURATION_SECONDS.set(request_duration);
}

pub fn get_metrics() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let registry = REGISTRY.get().expect("metrics registry not initialized");
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
