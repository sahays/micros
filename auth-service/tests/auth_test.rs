//! Authentication integration tests for auth-service v2.
//!
//! Tests registration, login, refresh, and logout flows.

mod common;

use common::{cleanup_test_data, TestApp};
use serde_json::json;
use uuid::Uuid;

/// Helper to create a test tenant, returns (tenant_id, tenant_slug)
async fn create_test_tenant(pool: &sqlx::PgPool, label: &str) -> (Uuid, String) {
    let tenant_id = Uuid::new_v4();
    let slug = format!("test-tenant-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_slug, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, $2, $3, 'active', now())
        "#,
    )
    .bind(tenant_id)
    .bind(&slug)
    .bind(label)
    .execute(pool)
    .await
    .expect("Failed to create test tenant");
    (tenant_id, slug)
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn register_new_user_succeeds() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("test-{}@example.com", Uuid::new_v4());
    let request_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": "SecurePass123!",
        "display_name": "Test User"
    });

    // Act
    let response = client
        .post(format!("{}/auth/register", app.address))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(
        body.get("user").is_some(),
        "Response missing 'user' field: {:?}",
        body
    );
    assert!(
        body["user"].get("user_id").is_some(),
        "Response missing 'user.user_id' field"
    );
    assert!(
        body.get("access_token").is_some(),
        "Response missing 'access_token' field"
    );
    assert!(
        body.get("refresh_token").is_some(),
        "Response missing 'refresh_token' field"
    );
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn register_duplicate_email_fails() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("duplicate-{}@example.com", Uuid::new_v4());
    let request_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": "SecurePass123!",
        "display_name": "Test User"
    });

    // Register first user
    let response = client
        .post(format!("{}/auth/register", app.address))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(response.status(), 201);

    // Act - Try to register with same email
    let response = client
        .post(format!("{}/auth/register", app.address))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 409); // Conflict
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn login_with_valid_credentials_succeeds() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("login-test-{}@example.com", Uuid::new_v4());
    let password = "SecurePass123!";

    // Register user first
    let register_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": password,
        "display_name": "Test User"
    });

    client
        .post(format!("{}/auth/register", app.address))
        .json(&register_body)
        .send()
        .await
        .expect("Failed to register user");

    // Act - Login
    let login_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": password
    });

    let response = client
        .post(format!("{}/auth/login", app.address))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("access_token").is_some());
    assert!(body.get("refresh_token").is_some());
    assert!(body.get("expires_in").is_some());
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn login_with_wrong_password_fails() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("wrong-pass-{}@example.com", Uuid::new_v4());

    // Register user first
    let register_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": "CorrectPassword123!",
        "display_name": "Test User"
    });

    client
        .post(format!("{}/auth/register", app.address))
        .json(&register_body)
        .send()
        .await
        .expect("Failed to register user");

    // Act - Login with wrong password
    let login_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": "WrongPassword456!"
    });

    let response = client
        .post(format!("{}/auth/login", app.address))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 401);
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn login_with_nonexistent_user_fails() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    // Act - Login without registering
    let login_body = json!({
        "tenant_slug": tenant_slug,
        "email": "nonexistent@example.com",
        "password": "SomePassword123!"
    });

    let response = client
        .post(format!("{}/auth/login", app.address))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 401);
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn refresh_token_returns_new_tokens() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("refresh-test-{}@example.com", Uuid::new_v4());
    let password = "SecurePass123!";

    // Register user
    let register_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": password,
        "display_name": "Test User"
    });

    let register_response = client
        .post(format!("{}/auth/register", app.address))
        .json(&register_body)
        .send()
        .await
        .expect("Failed to register user");

    let register_body: serde_json::Value = register_response
        .json()
        .await
        .expect("Failed to parse register response");
    let refresh_token = register_body["refresh_token"].as_str().unwrap();

    // Act - Refresh token
    let refresh_body = json!({
        "refresh_token": refresh_token
    });

    let response = client
        .post(format!("{}/auth/refresh", app.address))
        .json(&refresh_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("access_token").is_some());
    assert!(body.get("refresh_token").is_some());
    // New refresh token should be different
    assert_ne!(body["refresh_token"].as_str().unwrap(), refresh_token);
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn logout_invalidates_session() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    let (_tenant_id, tenant_slug) = create_test_tenant(&app.pool, "Test Tenant").await;
    let client = app.client();

    let email = format!("logout-test-{}@example.com", Uuid::new_v4());
    let password = "SecurePass123!";

    // Register user
    let register_body = json!({
        "tenant_slug": tenant_slug,
        "email": email,
        "password": password,
        "display_name": "Test User"
    });

    let register_response = client
        .post(format!("{}/auth/register", app.address))
        .json(&register_body)
        .send()
        .await
        .expect("Failed to register user");

    let register_body: serde_json::Value = register_response
        .json()
        .await
        .expect("Failed to parse register response");
    let refresh_token = register_body["refresh_token"].as_str().unwrap();

    // Act - Logout
    let logout_body = json!({
        "refresh_token": refresh_token
    });

    let response = client
        .post(format!("{}/auth/logout", app.address))
        .json(&logout_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    // Try to use the same refresh token - should fail
    let refresh_body = json!({
        "refresh_token": refresh_token
    });

    let response = client
        .post(format!("{}/auth/refresh", app.address))
        .json(&refresh_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 401);
}
