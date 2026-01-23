//! Prometheus metrics for genai-service.
//!
//! Provides gRPC and AI-specific metrics for observability.

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::OnceLock;

// Global registry
pub static REGISTRY: OnceLock<Registry> = OnceLock::new();

// gRPC metrics
pub static GRPC_REQUESTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static GRPC_REQUEST_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();
pub static GRPC_REQUESTS_IN_FLIGHT: OnceLock<IntGaugeVec> = OnceLock::new();

// AI-specific metrics
pub static GENAI_TOKENS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static GENAI_REQUESTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static GENAI_PROVIDER_LATENCY_SECONDS: OnceLock<HistogramVec> = OnceLock::new();
pub static GENAI_PROVIDER_ERRORS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

// Database metrics
pub static DB_OPERATION_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();
pub static DB_ERRORS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

// Document fetcher metrics
pub static DOCUMENT_FETCH_DURATION_SECONDS: OnceLock<HistogramVec> = OnceLock::new();
pub static DOCUMENT_FETCH_ERRORS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Initialize all metrics. Must be called once at startup.
pub fn init_metrics() {
    let registry = Registry::new();

    // gRPC request counter
    let grpc_requests_total = IntCounterVec::new(
        Opts::new("grpc_requests_total", "Total number of gRPC requests"),
        &["method", "status"],
    )
    .expect("Failed to create grpc_requests_total metric");

    // gRPC request duration histogram
    let grpc_request_duration = HistogramVec::new(
        HistogramOpts::new(
            "grpc_request_duration_seconds",
            "gRPC request duration in seconds",
        )
        .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]),
        &["method"],
    )
    .expect("Failed to create grpc_request_duration_seconds metric");

    // gRPC requests in flight
    let grpc_in_flight = IntGaugeVec::new(
        Opts::new(
            "grpc_requests_in_flight",
            "Number of gRPC requests currently being processed",
        ),
        &["method"],
    )
    .expect("Failed to create grpc_requests_in_flight metric");

    // Token counter (input/output by model and tenant for billing)
    let genai_tokens = IntCounterVec::new(
        Opts::new("genai_tokens_total", "Total tokens processed"),
        &["tenant_id", "model", "type"], // type: input, output
    )
    .expect("Failed to create genai_tokens_total metric");

    // AI request counter (with tenant for billing)
    let genai_requests = IntCounterVec::new(
        Opts::new("genai_requests_total", "Total GenAI requests"),
        &["tenant_id", "output_format", "model", "finish_reason"],
    )
    .expect("Failed to create genai_requests_total metric");

    // Provider latency histogram
    let provider_latency = HistogramVec::new(
        HistogramOpts::new(
            "genai_provider_latency_seconds",
            "AI provider API latency in seconds",
        )
        .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0]),
        &["provider", "model"],
    )
    .expect("Failed to create genai_provider_latency_seconds metric");

    // Provider error counter
    let provider_errors = IntCounterVec::new(
        Opts::new("genai_provider_errors_total", "Total AI provider errors"),
        &["provider", "error_type"],
    )
    .expect("Failed to create genai_provider_errors_total metric");

    // Database operation duration
    let db_duration = HistogramVec::new(
        HistogramOpts::new(
            "db_operation_duration_seconds",
            "Database operation duration in seconds",
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        &["operation", "collection"],
    )
    .expect("Failed to create db_operation_duration_seconds metric");

    // Database error counter
    let db_errors = IntCounterVec::new(
        Opts::new("db_errors_total", "Total database errors"),
        &["operation", "collection"],
    )
    .expect("Failed to create db_errors_total metric");

    // Document fetch duration
    let doc_fetch_duration = HistogramVec::new(
        HistogramOpts::new(
            "document_fetch_duration_seconds",
            "Document service fetch duration in seconds",
        )
        .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["operation"],
    )
    .expect("Failed to create document_fetch_duration_seconds metric");

    // Document fetch error counter
    let doc_fetch_errors = IntCounterVec::new(
        Opts::new("document_fetch_errors_total", "Total document fetch errors"),
        &["error_type"],
    )
    .expect("Failed to create document_fetch_errors_total metric");

    // Register all metrics
    registry
        .register(Box::new(grpc_requests_total.clone()))
        .expect("Failed to register grpc_requests_total");
    registry
        .register(Box::new(grpc_request_duration.clone()))
        .expect("Failed to register grpc_request_duration_seconds");
    registry
        .register(Box::new(grpc_in_flight.clone()))
        .expect("Failed to register grpc_requests_in_flight");
    registry
        .register(Box::new(genai_tokens.clone()))
        .expect("Failed to register genai_tokens_total");
    registry
        .register(Box::new(genai_requests.clone()))
        .expect("Failed to register genai_requests_total");
    registry
        .register(Box::new(provider_latency.clone()))
        .expect("Failed to register genai_provider_latency_seconds");
    registry
        .register(Box::new(provider_errors.clone()))
        .expect("Failed to register genai_provider_errors_total");
    registry
        .register(Box::new(db_duration.clone()))
        .expect("Failed to register db_operation_duration_seconds");
    registry
        .register(Box::new(db_errors.clone()))
        .expect("Failed to register db_errors_total");
    registry
        .register(Box::new(doc_fetch_duration.clone()))
        .expect("Failed to register document_fetch_duration_seconds");
    registry
        .register(Box::new(doc_fetch_errors.clone()))
        .expect("Failed to register document_fetch_errors_total");

    // Initialize globals
    let _ = REGISTRY.set(registry);
    let _ = GRPC_REQUESTS_TOTAL.set(grpc_requests_total);
    let _ = GRPC_REQUEST_DURATION_SECONDS.set(grpc_request_duration);
    let _ = GRPC_REQUESTS_IN_FLIGHT.set(grpc_in_flight);
    let _ = GENAI_TOKENS_TOTAL.set(genai_tokens);
    let _ = GENAI_REQUESTS_TOTAL.set(genai_requests);
    let _ = GENAI_PROVIDER_LATENCY_SECONDS.set(provider_latency);
    let _ = GENAI_PROVIDER_ERRORS_TOTAL.set(provider_errors);
    let _ = DB_OPERATION_DURATION_SECONDS.set(db_duration);
    let _ = DB_ERRORS_TOTAL.set(db_errors);
    let _ = DOCUMENT_FETCH_DURATION_SECONDS.set(doc_fetch_duration);
    let _ = DOCUMENT_FETCH_ERRORS_TOTAL.set(doc_fetch_errors);

    tracing::info!("Prometheus metrics initialized");
}

/// Get metrics in Prometheus text format.
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
        tracing::error!(error = %e, "Failed to encode metrics");
        return format!("# Failed to encode metrics: {}\n", e);
    }

    match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to convert metrics to UTF-8");
            format!("# Failed to convert metrics to UTF-8: {}\n", e)
        }
    }
}

// Helper functions for recording metrics

/// Record a completed gRPC request.
pub fn record_grpc_request(method: &str, status: &str, duration_secs: f64) {
    if let Some(counter) = GRPC_REQUESTS_TOTAL.get() {
        counter.with_label_values(&[method, status]).inc();
    }
    if let Some(histogram) = GRPC_REQUEST_DURATION_SECONDS.get() {
        histogram
            .with_label_values(&[method])
            .observe(duration_secs);
    }
}

/// Increment gRPC requests in flight.
pub fn inc_grpc_in_flight(method: &str) {
    if let Some(gauge) = GRPC_REQUESTS_IN_FLIGHT.get() {
        gauge.with_label_values(&[method]).inc();
    }
}

/// Decrement gRPC requests in flight.
pub fn dec_grpc_in_flight(method: &str) {
    if let Some(gauge) = GRPC_REQUESTS_IN_FLIGHT.get() {
        gauge.with_label_values(&[method]).dec();
    }
}

/// Record token usage with tenant_id for billing.
pub fn record_tokens(tenant_id: &str, model: &str, input_tokens: i32, output_tokens: i32) {
    if let Some(counter) = GENAI_TOKENS_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, model, "input"])
            .inc_by(input_tokens as u64);
        counter
            .with_label_values(&[tenant_id, model, "output"])
            .inc_by(output_tokens as u64);
    }
}

/// Record a completed GenAI request with tenant_id for billing.
pub fn record_genai_request(
    tenant_id: &str,
    output_format: &str,
    model: &str,
    finish_reason: &str,
) {
    if let Some(counter) = GENAI_REQUESTS_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, output_format, model, finish_reason])
            .inc();
    }
}

/// Record provider latency.
pub fn record_provider_latency(provider: &str, model: &str, duration_secs: f64) {
    if let Some(histogram) = GENAI_PROVIDER_LATENCY_SECONDS.get() {
        histogram
            .with_label_values(&[provider, model])
            .observe(duration_secs);
    }
}

/// Record a provider error.
pub fn record_provider_error(provider: &str, error_type: &str) {
    if let Some(counter) = GENAI_PROVIDER_ERRORS_TOTAL.get() {
        counter.with_label_values(&[provider, error_type]).inc();
    }
}

/// Record database operation duration.
pub fn record_db_operation(operation: &str, collection: &str, duration_secs: f64) {
    if let Some(histogram) = DB_OPERATION_DURATION_SECONDS.get() {
        histogram
            .with_label_values(&[operation, collection])
            .observe(duration_secs);
    }
}

/// Record a database error.
pub fn record_db_error(operation: &str, collection: &str) {
    if let Some(counter) = DB_ERRORS_TOTAL.get() {
        counter.with_label_values(&[operation, collection]).inc();
    }
}

/// Record document fetch duration.
pub fn record_document_fetch(operation: &str, duration_secs: f64) {
    if let Some(histogram) = DOCUMENT_FETCH_DURATION_SECONDS.get() {
        histogram
            .with_label_values(&[operation])
            .observe(duration_secs);
    }
}

/// Record a document fetch error.
pub fn record_document_fetch_error(error_type: &str) {
    if let Some(counter) = DOCUMENT_FETCH_ERRORS_TOTAL.get() {
        counter.with_label_values(&[error_type]).inc();
    }
}
