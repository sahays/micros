//! Prometheus metrics for ledger-service.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_histogram_vec, CounterVec, HistogramVec, TextEncoder,
};

/// gRPC request counter by method and status.
pub static GRPC_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "ledger_grpc_requests_total",
        "Total number of gRPC requests",
        &["method", "status"]
    )
    .expect("Failed to register grpc_requests_total")
});

/// gRPC request duration histogram by method.
pub static GRPC_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "ledger_grpc_request_duration_seconds",
        "gRPC request duration in seconds",
        &["method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .expect("Failed to register grpc_request_duration")
});

/// Transaction counter (no high-cardinality labels).
pub static TRANSACTIONS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "ledger_transactions_total",
        "Total number of transactions posted",
        &["status"]  // ok, error - not tenant_id to avoid cardinality explosion
    )
    .expect("Failed to register transactions_total")
});

/// Error counter for alerting.
pub static ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "ledger_errors_total",
        "Total number of errors by type",
        &["error_type"]  // db_error, validation_error, etc.
    )
    .expect("Failed to register errors_total")
});

/// Account counter by type.
pub static ACCOUNTS_CREATED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "ledger_accounts_created_total",
        "Total number of accounts created",
        &["account_type"]
    )
    .expect("Failed to register accounts_created")
});

/// Database query duration histogram.
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "ledger_db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("Failed to register db_query_duration")
});

/// Initialize all metrics (forces lazy initialization).
pub fn init_metrics() {
    Lazy::force(&GRPC_REQUESTS_TOTAL);
    Lazy::force(&GRPC_REQUEST_DURATION);
    Lazy::force(&TRANSACTIONS_TOTAL);
    Lazy::force(&ACCOUNTS_CREATED);
    Lazy::force(&DB_QUERY_DURATION);
    Lazy::force(&ERRORS_TOTAL);
}

/// Get metrics in Prometheus text format.
pub fn get_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder
        .encode_to_string(&metric_families)
        .unwrap_or_default()
}
