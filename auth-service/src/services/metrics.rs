use prometheus::{Encoder, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder};
use std::sync::OnceLock;

// Global registry
pub static REGISTRY: OnceLock<Registry> = OnceLock::new();

// Metrics
pub static HTTP_REQUESTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static HTTP_REQUEST_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();

pub fn init_metrics() {
    let registry = Registry::new();

    let requests_total = match IntCounterVec::new(
        Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status"],
    ) {
        Ok(metric) => metric,
        Err(e) => {
            tracing::error!("Failed to create http_requests_total metric: {}", e);
            panic!("Failed to initialize metrics: {}", e);
        }
    };

    let request_duration = match HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        ),
        &["method", "path", "status"],
    ) {
        Ok(metric) => metric,
        Err(e) => {
            tracing::error!(
                "Failed to create http_request_duration_seconds metric: {}",
                e
            );
            panic!("Failed to initialize metrics: {}", e);
        }
    };

    if let Err(e) = registry.register(Box::new(requests_total.clone())) {
        tracing::error!("Failed to register http_requests_total collector: {}", e);
        panic!("Failed to initialize metrics: {}", e);
    }

    if let Err(e) = registry.register(Box::new(request_duration.clone())) {
        tracing::error!(
            "Failed to register http_request_duration_seconds collector: {}",
            e
        );
        panic!("Failed to initialize metrics: {}", e);
    }

    // Initialize globals
    let _ = REGISTRY.set(registry);
    let _ = HTTP_REQUESTS_TOTAL.set(requests_total);
    let _ = HTTP_REQUEST_DURATION_SECONDS.set(request_duration);
}

pub fn get_metrics() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    let registry = match REGISTRY.get() {
        Some(r) => r,
        None => {
            tracing::error!("Metrics registry not initialized");
            return "# Metrics registry not initialized\n".to_string();
        }
    };

    let metric_families = registry.gather();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return format!("# Failed to encode metrics: {}\n", e);
    }

    match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to convert metrics to UTF-8: {}", e);
            format!("# Failed to convert metrics to UTF-8: {}\n", e)
        }
    }
}
