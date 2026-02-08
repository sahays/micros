use reqwest::Client;
use workflow_tests::helpers::genai;

#[tokio::test]
async fn health_check_returns_ok() {
    let http_base = genai::http_base_url();
    let client = Client::new();

    let response = client
        .get(format!("{}/health", http_base))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "genai-service");
}

#[tokio::test]
async fn readiness_check_returns_ok() {
    let http_base = genai::http_base_url();
    let client = Client::new();

    let response = client
        .get(format!("{}/ready", http_base))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}
