//! Metrics collection and Prometheus export.
//!
//! Initializes the metrics exporter and provides the /metrics endpoint handler.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;

/// Global handle to the Prometheus recorder.
pub static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Initialize the metrics recorder.
///
/// This must be called once at startup before any metrics are recorded.
/// Panics if called more than once.
pub fn init_metrics() {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    if METRICS_HANDLE.set(handle).is_err() {
        panic!("failed to set metrics handle: already initialized");
    }
}

/// Get the current metrics in Prometheus text format.
///
/// Returns a string suitable for the /metrics HTTP endpoint.
pub fn get_metrics() -> String {
    METRICS_HANDLE
        .get()
        .map(|handle| handle.render())
        .unwrap_or_else(|| "# Metrics recorder not initialized".to_string())
}
