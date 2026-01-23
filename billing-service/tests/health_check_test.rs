//! Health check integration tests for billing-service.

mod common;

use common::TestApp;
use reqwest::Client;

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
    assert_eq!(body["service"], "billing-service");

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
async fn metrics_endpoint_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/metrics", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    // Metrics endpoint should return 200 OK with text/plain content type
    assert!(response.status().is_success());
    assert!(response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("").contains("text/plain"))
        .unwrap_or(false));

    app.cleanup().await;
}
