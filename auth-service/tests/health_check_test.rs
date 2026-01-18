//! Health check integration tests for auth-service v2.

mod common;

use common::TestApp;

#[tokio::test]
#[ignore] // Requires PostgreSQL and Redis
async fn health_check_returns_200() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    let client = app.client();

    // Act
    let response = client
        .get(format!("{}/health", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "auth-service-test");
    assert_eq!(body["checks"]["postgresql"], "up");
}
