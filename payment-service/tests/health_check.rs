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
    assert_eq!(body["service"], "payment-service");

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
