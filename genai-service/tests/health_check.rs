//! Integration tests for genai-service.
//!
//! These tests use mock providers and require MongoDB to be running.
//! Run with: cargo test -p genai-service --test health_check

use genai_service::config::GenaiConfig;
use genai_service::startup::Application;
use reqwest::Client;
use std::time::Duration;

/// Spawn the application on a random port and return the port number.
async fn spawn_app() -> u16 {
    // Set test environment variables
    std::env::set_var("ENVIRONMENT", "test");
    std::env::set_var("APP__PORT", "0"); // Random port
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    std::env::set_var("MONGODB_DATABASE", "genai_test_db");
    std::env::set_var("GOOGLE_API_KEY", "test-api-key");
    std::env::set_var("GENAI_TEXT_MODEL", "gemini-2.0-flash");
    std::env::set_var("GENAI_AUDIO_MODEL", "gemini-2.0-flash");
    std::env::set_var("GENAI_VIDEO_MODEL", "veo-2");

    let config = GenaiConfig::load().expect("Failed to load config");
    let app = Application::build(config)
        .await
        .expect("Failed to build application");

    let port = app.http_port();

    // Spawn the server in the background
    tokio::spawn(async move {
        let _ = app.run_until_stopped().await;
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    port
}

#[tokio::test]
async fn health_check_returns_ok() {
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(format!("http://localhost:{}/health", port))
        .timeout(Duration::from_secs(5))
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
    // Skip if MongoDB is not available
    if std::env::var("SKIP_MONGO_TESTS").is_ok() {
        eprintln!("Skipping test: SKIP_MONGO_TESTS is set");
        return;
    }

    let port = spawn_app().await;
    let client = Client::new();

    let response = client
        .get(format!("http://localhost:{}/ready", port))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}
