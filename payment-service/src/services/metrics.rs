use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;

pub static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn init_metrics() {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    if METRICS_HANDLE.set(handle).is_err() {
        panic!("failed to set metrics handle: already initialized");
    }
}

pub fn get_metrics() -> String {
    METRICS_HANDLE
        .get()
        .map(|handle| handle.render())
        .unwrap_or_else(|| "# Metrics recorder not initialized".to_string())
}
