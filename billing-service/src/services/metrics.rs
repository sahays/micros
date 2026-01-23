//! Metrics module for billing-service.
//! Provides Prometheus metrics for billing operations and per-tenant metering.

use once_cell::sync::Lazy;
use prometheus::{
    histogram_opts, opts, register_histogram_vec, register_int_counter_vec, Encoder, HistogramVec,
    IntCounterVec, TextEncoder,
};
use std::sync::OnceLock;

/// Database query duration histogram
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        histogram_opts!(
            "billing_db_query_duration_seconds",
            "Database query duration"
        ),
        &["operation"]
    )
    .expect("Failed to register DB_QUERY_DURATION")
});

/// Plan operations counter (per-tenant metering)
pub static PLAN_OPERATIONS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Subscription operations counter (per-tenant metering)
pub static SUBSCRIPTION_OPERATIONS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Usage records counter (per-tenant metering)
pub static USAGE_RECORDS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Billing runs counter (per-tenant metering)
pub static BILLING_RUNS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Charges created counter (per-tenant metering)
pub static CHARGES_CREATED_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// gRPC request counter
pub static GRPC_REQUESTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// gRPC request duration histogram
pub static GRPC_REQUEST_DURATION: OnceLock<HistogramVec> = OnceLock::new();

/// Error counter for alerting
pub static ERRORS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Charge amount counter by currency (monetary tracking)
pub static CHARGE_AMOUNT_TOTAL: OnceLock<prometheus::CounterVec> = OnceLock::new();

/// Initialize all metrics. Call once at startup.
pub fn init_metrics() {
    // Plan operations
    PLAN_OPERATIONS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!(
                "billing_plan_operations_total",
                "Total plan operations by tenant and operation type"
            ),
            &["tenant_id", "operation"]
        )
        .expect("Failed to register PLAN_OPERATIONS_TOTAL")
    });

    // Subscription operations
    SUBSCRIPTION_OPERATIONS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!(
                "billing_subscription_operations_total",
                "Total subscription operations by tenant and operation type"
            ),
            &["tenant_id", "operation"]
        )
        .expect("Failed to register SUBSCRIPTION_OPERATIONS_TOTAL")
    });

    // Usage records
    USAGE_RECORDS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!(
                "billing_usage_records_total",
                "Total usage records by tenant"
            ),
            &["tenant_id", "component_id"]
        )
        .expect("Failed to register USAGE_RECORDS_TOTAL")
    });

    // Billing runs
    BILLING_RUNS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!(
                "billing_runs_total",
                "Total billing runs by tenant and status"
            ),
            &["tenant_id", "run_type", "status"]
        )
        .expect("Failed to register BILLING_RUNS_TOTAL")
    });

    // Charges created
    CHARGES_CREATED_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!(
                "billing_charges_created_total",
                "Total charges created by tenant and type"
            ),
            &["tenant_id", "charge_type"]
        )
        .expect("Failed to register CHARGES_CREATED_TOTAL")
    });

    // gRPC requests
    GRPC_REQUESTS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!("billing_grpc_requests_total", "Total gRPC requests"),
            &["method", "status"]
        )
        .expect("Failed to register GRPC_REQUESTS_TOTAL")
    });

    // gRPC request duration with custom buckets for billing workloads
    GRPC_REQUEST_DURATION.get_or_init(|| {
        register_histogram_vec!(
            histogram_opts!(
                "billing_grpc_request_duration_seconds",
                "gRPC request duration",
                vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
            ),
            &["method"]
        )
        .expect("Failed to register GRPC_REQUEST_DURATION")
    });

    // Error counter for alerting
    ERRORS_TOTAL.get_or_init(|| {
        register_int_counter_vec!(
            opts!("billing_errors_total", "Total errors by type for alerting"),
            &["error_type", "method"]
        )
        .expect("Failed to register ERRORS_TOTAL")
    });

    // Charge amount counter for financial tracking
    CHARGE_AMOUNT_TOTAL.get_or_init(|| {
        prometheus::register_counter_vec!(
            prometheus::opts!(
                "billing_charge_amount_total",
                "Total charge amount by currency and type"
            ),
            &["tenant_id", "currency", "charge_type"]
        )
        .expect("Failed to register CHARGE_AMOUNT_TOTAL")
    });

    // Force initialization of lazy statics
    let _ = &*DB_QUERY_DURATION;
}

/// Get metrics in Prometheus text format.
pub fn get_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");
    String::from_utf8(buffer).expect("Failed to convert metrics to string")
}

/// Record a plan operation.
pub fn record_plan_operation(tenant_id: &str, operation: &str) {
    if let Some(counter) = PLAN_OPERATIONS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, operation]).inc();
    }
}

/// Record a subscription operation.
pub fn record_subscription_operation(tenant_id: &str, operation: &str) {
    if let Some(counter) = SUBSCRIPTION_OPERATIONS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, operation]).inc();
    }
}

/// Record a usage operation.
pub fn record_usage_operation(tenant_id: &str, component_id: &str) {
    if let Some(counter) = USAGE_RECORDS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, component_id]).inc();
    }
}

/// Record a billing run.
pub fn record_billing_run(tenant_id: &str, run_type: &str, status: &str) {
    if let Some(counter) = BILLING_RUNS_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, run_type, status])
            .inc();
    }
}

/// Record a charge created.
pub fn record_charge_created(tenant_id: &str, charge_type: &str) {
    if let Some(counter) = CHARGES_CREATED_TOTAL.get() {
        counter.with_label_values(&[tenant_id, charge_type]).inc();
    }
}

/// Record a gRPC request.
pub fn record_grpc_request(method: &str, status: &str) {
    if let Some(counter) = GRPC_REQUESTS_TOTAL.get() {
        counter.with_label_values(&[method, status]).inc();
    }
}

/// Record gRPC request duration.
pub fn record_grpc_request_duration(method: &str, duration_secs: f64) {
    if let Some(histogram) = GRPC_REQUEST_DURATION.get() {
        histogram
            .with_label_values(&[method])
            .observe(duration_secs);
    }
}

/// Record an error for alerting.
pub fn record_error(error_type: &str, method: &str) {
    if let Some(counter) = ERRORS_TOTAL.get() {
        counter.with_label_values(&[error_type, method]).inc();
    }
}

/// Record a charge amount for financial tracking.
pub fn record_charge_amount(tenant_id: &str, currency: &str, charge_type: &str, amount: f64) {
    if let Some(counter) = CHARGE_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency, charge_type])
            .inc_by(amount.abs());
    }
}
