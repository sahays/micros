//! Integration tests for AdminService Bootstrap functionality.
//!
//! Tests the first-time system setup flow.

mod common;

use auth_service::grpc::proto::auth::BootstrapRequest;
use common::{with_admin_key, TestApp};
use tonic::Request;

#[tokio::test]
async fn bootstrap_creates_tenant_and_superadmin() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: Some("System Admin".to_string()),
    }));

    let response = client.bootstrap(request).await;
    assert!(
        response.is_ok(),
        "Bootstrap should succeed: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();

    // Verify all IDs are returned
    assert!(!response.tenant_id.is_empty());
    assert!(!response.root_org_node_id.is_empty());
    assert!(!response.superadmin_role_id.is_empty());
    assert!(!response.admin_user_id.is_empty());

    // Verify tokens are returned
    assert!(!response.access_token.is_empty());
    assert!(!response.refresh_token.is_empty());

    // Clean up
    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_fails_without_admin_api_key() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    // Request without admin API key
    let request = Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    });

    let response = client.bootstrap(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert!(status.message().contains("X-Admin-Api-Key"));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_fails_with_invalid_admin_api_key() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    // Request with wrong admin API key
    let mut request = Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    });
    request
        .metadata_mut()
        .insert("x-admin-api-key", "wrong-key".parse().unwrap());

    let response = client.bootstrap(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_fails_on_second_call() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    // First bootstrap should succeed
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await;
    assert!(response.is_ok());

    // Second bootstrap should fail
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "another".to_string(),
        tenant_label: "Another Corp".to_string(),
        admin_email: "admin@another.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    assert!(status.message().contains("Bootstrap already completed"));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_validates_password_requirements() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    // Password too short
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "short".to_string(), // Too short
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("admin_password"));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_validates_required_fields() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    // Missing tenant_slug
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await;
    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("tenant_slug"));

    // Missing admin_email
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await;
    assert!(response.is_err());
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("admin_email"));

    app.cleanup().await.unwrap();
}

#[tokio::test]
async fn bootstrap_returns_valid_access_token() {
    let app = TestApp::spawn().await;
    let mut client = app.admin_client().await;

    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: "acme".to_string(),
        tenant_label: "Acme Corporation".to_string(),
        admin_email: "admin@acme.com".to_string(),
        admin_password: "SecurePass123!".to_string(),
        admin_display_name: None,
    }));

    let response = client.bootstrap(request).await.unwrap().into_inner();

    // Verify the access token is valid by using it to call a protected endpoint
    let mut org_client = app.org_client().await;

    let list_request = common::with_auth(
        Request::new(auth_service::grpc::proto::auth::ListTenantOrgNodesRequest {
            tenant_id: response.tenant_id.clone(),
        }),
        &response.access_token,
    );

    // Superadmin should be able to list org nodes
    let list_response = org_client.list_tenant_org_nodes(list_request).await;
    assert!(
        list_response.is_ok(),
        "Superadmin should be able to list org nodes: {:?}",
        list_response.err()
    );

    app.cleanup().await.unwrap();
}
