use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use prometheus::{IntCounterVec, Opts, Registry};
use std::sync::OnceLock;

pub static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
pub static PROMETHEUS_REGISTRY: OnceLock<Registry> = OnceLock::new();
pub static PAYMENT_TRANSACTIONS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_AMOUNT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

pub fn init_metrics() {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    if METRICS_HANDLE.set(handle).is_err() {
        panic!("failed to set metrics handle: already initialized");
    }

    // Initialize Prometheus registry for custom metrics
    let registry = Registry::new();

    // Transaction counter with tenant_id for billing
    let transactions_counter = IntCounterVec::new(
        Opts::new(
            "payment_transactions_total",
            "Total payment transactions by tenant and status",
        ),
        &["tenant_id", "status"],
    )
    .expect("Failed to create payment_transactions_total metric");

    // Amount counter with tenant_id for billing (in smallest currency unit)
    let amount_counter = IntCounterVec::new(
        Opts::new(
            "payment_amount_total",
            "Total payment amounts by tenant and currency (in smallest unit)",
        ),
        &["tenant_id", "currency"],
    )
    .expect("Failed to create payment_amount_total metric");

    registry
        .register(Box::new(transactions_counter.clone()))
        .expect("Failed to register payment_transactions_total");
    registry
        .register(Box::new(amount_counter.clone()))
        .expect("Failed to register payment_amount_total");

    PROMETHEUS_REGISTRY
        .set(registry)
        .expect("Failed to set prometheus registry");
    PAYMENT_TRANSACTIONS_TOTAL
        .set(transactions_counter)
        .expect("Failed to set payment_transactions_total");
    PAYMENT_AMOUNT_TOTAL
        .set(amount_counter)
        .expect("Failed to set payment_amount_total");
}

pub fn get_metrics() -> String {
    let mut output = METRICS_HANDLE
        .get()
        .map(|handle| handle.render())
        .unwrap_or_else(|| "# Metrics recorder not initialized\n".to_string());

    // Append custom prometheus metrics
    if let Some(registry) = PROMETHEUS_REGISTRY.get() {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        if let Ok(custom_metrics) = String::from_utf8(buffer) {
            output.push_str(&custom_metrics);
        }
    }

    output
}

/// Record a payment transaction for billing/metering.
pub fn record_transaction(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_TRANSACTIONS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

/// Record payment amount for billing/metering.
pub fn record_amount(tenant_id: &str, currency: &str, amount_cents: u64) {
    if let Some(counter) = PAYMENT_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency])
            .inc_by(amount_cents);
    }
}
