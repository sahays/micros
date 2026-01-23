mod common;

use common::TestApp;
use document_service::services::init_metrics;
use reqwest::Client;
use std::sync::Once;

// Initialize metrics once for all tests
static INIT_METRICS: Once = Once::new();

fn ensure_metrics_initialized() {
    INIT_METRICS.call_once(|| {
        init_metrics();
    });
}

#[tokio::test]
async fn health_check_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/health", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "document-service");

    app.cleanup().await;
}

#[tokio::test]
async fn readiness_check_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/ready", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());

    app.cleanup().await;
}

#[tokio::test]
async fn metrics_endpoint_returns_prometheus_format() {
    ensure_metrics_initialized();
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/metrics", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());

    let content_type = response
        .headers()
        .get("content-type")
        .expect("Missing content-type header")
        .to_str()
        .expect("Invalid content-type");

    assert!(content_type.starts_with("text/plain"));

    let body = response.text().await.expect("Failed to get response body");
    // Prometheus metrics format starts with # HELP or metric names
    // If metrics are recorded, we should see some output
    // The body might be empty if no metrics recorded yet, which is also valid
    assert!(
        body.is_empty() || body.contains('#') || body.contains('_'),
        "Unexpected metrics format: {}",
        body
    );

    app.cleanup().await;
}
