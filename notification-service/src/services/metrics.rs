//! Metrics collection for notification-service.
//!
//! Provides per-tenant billing metrics and standard Prometheus metrics.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use prometheus::{IntCounterVec, Opts, Registry};
use std::sync::OnceLock;

pub static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
pub static PROMETHEUS_REGISTRY: OnceLock<Registry> = OnceLock::new();
pub static NOTIFICATION_SENT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static NOTIFICATION_PROVIDER_CALLS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Initialize metrics collection.
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

    // Notification counter with tenant_id for billing
    let notifications_counter = IntCounterVec::new(
        Opts::new(
            "notification_sent_total",
            "Total notifications sent by tenant, channel, and status",
        ),
        &["tenant_id", "channel", "status"],
    )
    .expect("Failed to create notification_sent_total metric");

    // Provider call counter for monitoring
    let provider_calls_counter = IntCounterVec::new(
        Opts::new(
            "notification_provider_calls_total",
            "Total provider API calls by provider and status",
        ),
        &["provider", "status"],
    )
    .expect("Failed to create notification_provider_calls_total metric");

    registry
        .register(Box::new(notifications_counter.clone()))
        .expect("Failed to register notification_sent_total");
    registry
        .register(Box::new(provider_calls_counter.clone()))
        .expect("Failed to register notification_provider_calls_total");

    PROMETHEUS_REGISTRY
        .set(registry)
        .expect("Failed to set prometheus registry");
    NOTIFICATION_SENT_TOTAL
        .set(notifications_counter)
        .expect("Failed to set notification_sent_total");
    NOTIFICATION_PROVIDER_CALLS_TOTAL
        .set(provider_calls_counter)
        .expect("Failed to set notification_provider_calls_total");
}

/// Get metrics output in Prometheus text format.
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

/// Record a notification for billing/metering.
pub fn record_notification(tenant_id: &str, channel: &str, status: &str) {
    if let Some(counter) = NOTIFICATION_SENT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, channel, status])
            .inc();
    }
}

/// Record a provider API call.
pub fn record_provider_call(provider: &str, status: &str) {
    if let Some(counter) = NOTIFICATION_PROVIDER_CALLS_TOTAL.get() {
        counter.with_label_values(&[provider, status]).inc();
    }
}
