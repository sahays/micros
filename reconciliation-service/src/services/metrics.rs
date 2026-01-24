//! Prometheus metrics for reconciliation-service.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_histogram_vec, CounterVec, Encoder, HistogramVec, TextEncoder,
};

/// Counter for gRPC requests by method and status.
pub static GRPC_REQUESTS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "reconciliation_grpc_requests_total",
        "Total number of gRPC requests",
        &["method", "status"]
    )
    .expect("Failed to register GRPC_REQUESTS")
});

/// Histogram for gRPC request duration by method.
pub static GRPC_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "reconciliation_grpc_request_duration_seconds",
        "gRPC request duration in seconds",
        &["method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to register GRPC_REQUEST_DURATION")
});

/// Counter for database query duration.
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "reconciliation_db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5]
    )
    .expect("Failed to register DB_QUERY_DURATION")
});

/// Counter for reconciliation operations.
pub static RECONCILIATION_OPERATIONS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "reconciliation_operations_total",
        "Total number of reconciliation operations",
        &["operation", "status"]
    )
    .expect("Failed to register RECONCILIATION_OPERATIONS")
});

/// Counter for statement imports.
pub static STATEMENT_IMPORTS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "reconciliation_statement_imports_total",
        "Total number of statement imports",
        &["status"]
    )
    .expect("Failed to register STATEMENT_IMPORTS")
});

/// Counter for transaction matches.
pub static TRANSACTION_MATCHES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "reconciliation_transaction_matches_total",
        "Total number of transaction matches",
        &["match_type"]
    )
    .expect("Failed to register TRANSACTION_MATCHES")
});

/// Counter for errors.
pub static ERRORS: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "reconciliation_errors_total",
        "Total number of errors",
        &["error_type"]
    )
    .expect("Failed to register ERRORS")
});

/// Initialize all metrics (forces lazy initialization).
pub fn init_metrics() {
    Lazy::force(&GRPC_REQUESTS);
    Lazy::force(&GRPC_REQUEST_DURATION);
    Lazy::force(&DB_QUERY_DURATION);
    Lazy::force(&RECONCILIATION_OPERATIONS);
    Lazy::force(&STATEMENT_IMPORTS);
    Lazy::force(&TRANSACTION_MATCHES);
    Lazy::force(&ERRORS);
}

/// Get all metrics as Prometheus text format.
pub fn get_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Record a gRPC request.
pub fn record_grpc_request(method: &str, status: &str) {
    GRPC_REQUESTS.with_label_values(&[method, status]).inc();
}

/// Record gRPC request duration.
pub fn record_grpc_request_duration(method: &str, duration_secs: f64) {
    GRPC_REQUEST_DURATION
        .with_label_values(&[method])
        .observe(duration_secs);
}

/// Record an error.
pub fn record_error(error_type: &str) {
    ERRORS.with_label_values(&[error_type]).inc();
}

/// Record a reconciliation operation.
pub fn record_reconciliation_operation(operation: &str, status: &str) {
    RECONCILIATION_OPERATIONS
        .with_label_values(&[operation, status])
        .inc();
}

/// Record a statement import.
pub fn record_statement_import(status: &str) {
    STATEMENT_IMPORTS.with_label_values(&[status]).inc();
}

/// Record a transaction match.
pub fn record_transaction_match(match_type: &str) {
    TRANSACTION_MATCHES.with_label_values(&[match_type]).inc();
}
