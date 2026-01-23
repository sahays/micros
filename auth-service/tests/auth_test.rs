//! Integration tests for AuthService authentication flows.
//!
//! Tests registration, login, token refresh, and logout.

mod common;

use auth_service::grpc::proto::auth::{
    BootstrapRequest, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
    ValidateTokenRequest,
};
use common::{with_admin_key, TestApp};
use tonic::Request;

/// Helper to bootstrap and get a tenant for auth tests.
async fn setup_tenant(app: &TestApp) -> String {
    let mut client = app.admin_client().await;
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "authtest".to_string(),
        tenant_label: "Auth Test Tenant".to_string(),
        admin_email: "admin@authtest.com".to_string(),
        admin_password: "AdminPass123!".to_string(),
        admin_display_name: None,
    }));
    client.bootstrap(request).await.unwrap();
    "authtest".to_string()
}

// ============================================================================
// Registration Tests
// ============================================================================

#[tokio::test]
async fn register_creates_new_user() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.clone(),
        email: "newuser@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: Some("New User".to_string()),
    });

    let response = client.register(request).await;
    assert!(
        response.is_ok(),
        "Register should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    assert_eq!(response.token_type, "Bearer");
    assert!(response.expires_in > 0);

    let user = response.user.unwrap();
    assert_eq!(user.email, "newuser@example.com");
    assert_eq!(user.display_name, Some("New User".to_string()));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn register_fails_for_duplicate_email() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register first user
    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.clone(),
        email: "duplicate@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    client.register(request).await.unwrap();

    // Try to register same email again
    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.clone(),
        email: "duplicate@example.com".to_string(),
        password: "DifferentPass123!".to_string(),
        display_name: None,
    });
    let response = client.register(request).await;

    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::AlreadyExists);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn register_fails_for_invalid_tenant() {
    let app = TestApp::spawn().await;
    let mut client = app.auth_client().await;

    let request = Request::new(RegisterRequest {
        tenant_slug: "nonexistent".to_string(),
        email: "user@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });

    let response = client.register(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn register_validates_password_length() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "user@example.com".to_string(),
        password: "short".to_string(), // Too short
        display_name: None,
    });

    let response = client.register(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("Password"));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn register_validates_email_format() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "invalid-email".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });

    let response = client.register(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().to_lowercase().contains("email"));

    app.cleanup().await.unwrap();
}

// ============================================================================
// Login Tests
// ============================================================================

#[tokio::test]
async fn login_succeeds_with_valid_credentials() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register user first
    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.clone(),
        email: "logintest@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    client.register(request).await.unwrap();

    // Login
    let request = Request::new(LoginRequest {
        tenant_slug: tenant_slug.clone(),
        email: "logintest@example.com".to_string(),
        password: "Password123!".to_string(),
    });

    let response = client.login(request).await;
    assert!(
        response.is_ok(),
        "Login should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    assert_eq!(response.token_type, "Bearer");

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn login_fails_with_wrong_password() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register user
    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.clone(),
        email: "wrongpass@example.com".to_string(),
        password: "CorrectPass123!".to_string(),
        display_name: None,
    });
    client.register(request).await.unwrap();

    // Login with wrong password
    let request = Request::new(LoginRequest {
        tenant_slug: tenant_slug.clone(),
        email: "wrongpass@example.com".to_string(),
        password: "WrongPass123!".to_string(),
    });

    let response = client.login(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn login_fails_for_nonexistent_user() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    let request = Request::new(LoginRequest {
        tenant_slug,
        email: "nonexistent@example.com".to_string(),
        password: "Password123!".to_string(),
    });

    let response = client.login(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await.unwrap();
}

// ============================================================================
// Token Refresh Tests
// ============================================================================

#[tokio::test]
async fn refresh_returns_new_tokens() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register and get tokens
    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "refresh@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let register_response = client.register(request).await.unwrap().into_inner();
    let original_refresh_token = register_response.refresh_token;

    // Refresh tokens
    let request = Request::new(RefreshRequest {
        refresh_token: original_refresh_token.clone(),
    });

    let response = client.refresh(request).await;
    assert!(
        response.is_ok(),
        "Refresh should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());
    // New refresh token should be different (rotation)
    assert_ne!(response.refresh_token, original_refresh_token);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn refresh_fails_with_invalid_token() {
    let app = TestApp::spawn().await;
    let mut client = app.auth_client().await;

    let request = Request::new(RefreshRequest {
        refresh_token: "invalid.token.here".to_string(),
    });

    let response = client.refresh(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn refresh_fails_after_logout() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register and get tokens
    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "logout@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let register_response = client.register(request).await.unwrap().into_inner();
    let refresh_token = register_response.refresh_token;

    // Logout
    let request = Request::new(LogoutRequest {
        refresh_token: refresh_token.clone(),
    });
    client.logout(request).await.unwrap();

    // Try to refresh with revoked token
    let request = Request::new(RefreshRequest {
        refresh_token: refresh_token.clone(),
    });

    let response = client.refresh(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await.unwrap();
}

// ============================================================================
// Token Validation Tests
// ============================================================================

#[tokio::test]
async fn validate_token_returns_claims_for_valid_token() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register and get token
    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "validate@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let register_response = client.register(request).await.unwrap().into_inner();

    // Validate the token
    let request = Request::new(ValidateTokenRequest {
        access_token: register_response.access_token,
    });

    let response = client.validate_token(request).await;
    assert!(
        response.is_ok(),
        "ValidateToken should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(response.valid);
    assert!(response.claims.is_some());

    let claims = response.claims.unwrap();
    assert!(!claims.sub.is_empty()); // User ID
    assert_eq!(claims.email, "validate@example.com");

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn validate_token_returns_invalid_for_bad_token() {
    let app = TestApp::spawn().await;
    let mut client = app.auth_client().await;

    let request = Request::new(ValidateTokenRequest {
        access_token: "invalid.token.here".to_string(),
    });

    let response = client.validate_token(request).await;
    assert!(response.is_ok()); // Should not error, just return valid=false

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
    assert!(response.claims.is_none());

    app.cleanup().await.unwrap();
}

// ============================================================================
// Logout Tests
// ============================================================================

#[tokio::test]
async fn logout_succeeds() {
    let app = TestApp::spawn().await;
    let tenant_slug = setup_tenant(&app).await;
    let mut client = app.auth_client().await;

    // Register and get tokens
    let request = Request::new(RegisterRequest {
        tenant_slug,
        email: "logouttest@example.com".to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let register_response = client.register(request).await.unwrap().into_inner();

    // Logout
    let request = Request::new(LogoutRequest {
        refresh_token: register_response.refresh_token,
    });

    let response = client.logout(request).await;
    assert!(
        response.is_ok(),
        "Logout should succeed: {:?}",
        response.err()
    );

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn logout_succeeds_with_invalid_token() {
    // Logout should be idempotent - it shouldn't fail for invalid tokens
    let app = TestApp::spawn().await;
    let mut client = app.auth_client().await;

    let request = Request::new(LogoutRequest {
        refresh_token: "invalid.token".to_string(),
    });

    let response = client.logout(request).await;
    // Should not fail even with invalid token
    assert!(response.is_ok());

    app.cleanup().await.unwrap();
}
