use document_service::config::DocumentConfig;
use document_service::startup::Application;
use axum::http::StatusCode;

#[tokio::test]
async fn health_check_works() {
    // 1. Setup
    // Load config (uses defaults for test if .env is missing or sets standard values)
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0; // Use random port for test

    let app = Application::build(config).await.expect("Failed to build application");
    let port = app.port();

    // Run the server in a background task
    tokio::spawn(app.run_until_stopped());

    // 2. Request
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .expect("Failed to execute request.");

    // 3. Assert
    assert_eq!(StatusCode::OK, response.status());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "document-service");
}
