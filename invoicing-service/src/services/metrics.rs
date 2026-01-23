//! Prometheus metrics for invoicing-service.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_histogram_vec, CounterVec, HistogramVec, TextEncoder,
};

/// gRPC request counter by method and status.
pub static GRPC_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_grpc_requests_total",
        "Total number of gRPC requests",
        &["method", "status"]
    )
    .expect("Failed to register grpc_requests_total")
});

/// gRPC request duration histogram by method.
pub static GRPC_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "invoicing_grpc_request_duration_seconds",
        "gRPC request duration in seconds",
        &["method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .expect("Failed to register grpc_request_duration")
});

/// Invoice counter by status.
pub static INVOICES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_invoices_total",
        "Total number of invoices by status",
        &["status"] // draft, issued, paid, void
    )
    .expect("Failed to register invoices_total")
});

/// Receipt counter.
pub static RECEIPTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_receipts_total",
        "Total number of receipts by payment method",
        &["payment_method"]
    )
    .expect("Failed to register receipts_total")
});

/// Error counter for alerting.
pub static ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_errors_total",
        "Total number of errors by type",
        &["error_type"]
    )
    .expect("Failed to register errors_total")
});

/// Database query duration histogram.
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "invoicing_db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("Failed to register db_query_duration")
});

/// Monetary amount counter by currency.
pub static INVOICE_AMOUNT_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_invoice_amount_total",
        "Total invoice amount by currency",
        &["currency"]
    )
    .expect("Failed to register invoice_amount_total")
});

/// Payment amount counter by currency.
pub static PAYMENT_AMOUNT_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "invoicing_payment_amount_total",
        "Total payment amount by currency",
        &["currency"]
    )
    .expect("Failed to register payment_amount_total")
});

/// Initialize all metrics (forces lazy initialization).
pub fn init_metrics() {
    Lazy::force(&GRPC_REQUESTS_TOTAL);
    Lazy::force(&GRPC_REQUEST_DURATION);
    Lazy::force(&INVOICES_TOTAL);
    Lazy::force(&RECEIPTS_TOTAL);
    Lazy::force(&ERRORS_TOTAL);
    Lazy::force(&DB_QUERY_DURATION);
    Lazy::force(&INVOICE_AMOUNT_TOTAL);
    Lazy::force(&PAYMENT_AMOUNT_TOTAL);
}

/// Get metrics in Prometheus text format.
pub fn get_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder
        .encode_to_string(&metric_families)
        .unwrap_or_default()
}
