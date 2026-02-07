//! Integration tests for capability-based access control.
//!
//! Tests that protected endpoints require appropriate capabilities.

mod common;

use auth_service::grpc::proto::auth::{
    assignment_service_client::AssignmentServiceClient, BootstrapRequest, CreateAssignmentRequest,
    CreateOrgNodeRequest, CreateRoleRequest, ListTenantOrgNodesRequest, ListTenantRolesRequest,
    LoginRequest, RegisterRequest,
};
use common::{with_admin_key, with_auth, TestApp};
use serial_test::serial;
use tonic::transport::Channel;
use tonic::Request;
use uuid::Uuid;

fn unique_slug(prefix: &str) -> String {
    format!("{}-{}", prefix, &Uuid::new_v4().to_string()[..8])
}

/// Bootstrap and return (tenant_id, root_org_node_id, superadmin_access_token, tenant_slug).
async fn setup_with_superadmin(app: &TestApp) -> (String, String, String, String) {
    let slug = unique_slug("captest");
    let mut client = app.admin_client().await;
    let request = with_admin_key(Request::new(BootstrapRequest {
        tenant_slug: slug.clone(),
        tenant_label: "Capability Test Tenant".to_string(),
        admin_email: format!("admin@{}.com", slug),
        admin_password: "AdminPass123!".to_string(),
        admin_display_name: None,
    }));
    let response = client.bootstrap(request).await.unwrap().into_inner();
    (
        response.tenant_id,
        response.root_org_node_id,
        response.access_token,
        slug,
    )
}

/// Register a regular user (no capabilities) and return access token.
async fn create_regular_user(app: &TestApp, tenant_slug: &str, email: &str) -> String {
    let mut client = app.auth_client().await;
    let request = Request::new(RegisterRequest {
        tenant_slug: tenant_slug.to_string(),
        email: email.to_string(),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let response = client.register(request).await.unwrap().into_inner();
    response.access_token
}

async fn create_assignment_client(port: u16) -> AssignmentServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = tonic::transport::Channel::from_shared(addr)
        .unwrap()
        .connect()
        .await
        .unwrap();
    AssignmentServiceClient::new(channel)
}

// ============================================================================
// Unauthenticated Access Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn protected_endpoint_rejects_unauthenticated_request() {
    let app = TestApp::spawn().await;
    let (tenant_id, _, _, _slug) = setup_with_superadmin(&app).await;
    let mut client = app.org_client().await;

    // Try to list org nodes without authentication
    let request = Request::new(ListTenantOrgNodesRequest {
        tenant_id: tenant_id.clone(),
    });

    let response = client.list_tenant_org_nodes(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
    assert!(status.message().contains("authorization"));

    let _ = app.cleanup().await;
}

// ============================================================================
// Superadmin Access Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn superadmin_can_access_all_protected_endpoints() {
    let app = TestApp::spawn().await;
    let (tenant_id, root_org_id, superadmin_token, _slug) = setup_with_superadmin(&app).await;

    // Test OrgService - list org nodes
    let mut org_client = app.org_client().await;
    let request = with_auth(
        Request::new(ListTenantOrgNodesRequest {
            tenant_id: tenant_id.clone(),
        }),
        &superadmin_token,
    );
    let response = org_client.list_tenant_org_nodes(request).await;
    assert!(
        response.is_ok(),
        "Superadmin should list org nodes: {:?}",
        response.err()
    );

    // Test OrgService - create org node
    let request = with_auth(
        Request::new(CreateOrgNodeRequest {
            tenant_id: tenant_id.clone(),
            parent_org_node_id: Some(root_org_id.clone()),
            node_type_code: "department".to_string(),
            node_label: "Engineering".to_string(),
        }),
        &superadmin_token,
    );
    let response = org_client.create_org_node(request).await;
    assert!(
        response.is_ok(),
        "Superadmin should create org node: {:?}",
        response.err()
    );

    // Test RoleService - list roles
    let mut role_client = app.role_client().await;
    let request = with_auth(
        Request::new(ListTenantRolesRequest {
            tenant_id: tenant_id.clone(),
        }),
        &superadmin_token,
    );
    let response = role_client.list_tenant_roles(request).await;
    assert!(
        response.is_ok(),
        "Superadmin should list roles: {:?}",
        response.err()
    );

    // Test RoleService - create role
    let request = with_auth(
        Request::new(CreateRoleRequest {
            tenant_id: tenant_id.clone(),
            role_label: "Test Role".to_string(),
        }),
        &superadmin_token,
    );
    let response = role_client.create_role(request).await;
    assert!(
        response.is_ok(),
        "Superadmin should create role: {:?}",
        response.err()
    );

    let _ = app.cleanup().await;
}

// ============================================================================
// Regular User (No Capabilities) Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn regular_user_cannot_create_org_node() {
    let app = TestApp::spawn().await;
    let (tenant_id, root_org_id, _, slug) = setup_with_superadmin(&app).await;

    // Create a regular user (no capabilities assigned)
    let user_token = create_regular_user(&app, &slug, &format!("user@{}.com", slug)).await;

    let mut client = app.org_client().await;
    let request = with_auth(
        Request::new(CreateOrgNodeRequest {
            tenant_id: tenant_id.clone(),
            parent_org_node_id: Some(root_org_id.clone()),
            node_type_code: "department".to_string(),
            node_label: "Unauthorized Department".to_string(),
        }),
        &user_token,
    );

    let response = client.create_org_node(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::PermissionDenied);
    assert!(status.message().contains("capability"));

    let _ = app.cleanup().await;
}

#[tokio::test]
#[serial]
async fn regular_user_cannot_list_org_nodes() {
    let app = TestApp::spawn().await;
    let (tenant_id, _, _, slug) = setup_with_superadmin(&app).await;

    let user_token = create_regular_user(&app, &slug, &format!("reader@{}.com", slug)).await;

    let mut client = app.org_client().await;
    let request = with_auth(
        Request::new(ListTenantOrgNodesRequest {
            tenant_id: tenant_id.clone(),
        }),
        &user_token,
    );

    let response = client.list_tenant_org_nodes(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::PermissionDenied);

    let _ = app.cleanup().await;
}

#[tokio::test]
#[serial]
async fn regular_user_cannot_create_role() {
    let app = TestApp::spawn().await;
    let (tenant_id, _, _, slug) = setup_with_superadmin(&app).await;

    let user_token = create_regular_user(&app, &slug, &format!("roleuser@{}.com", slug)).await;

    let mut client = app.role_client().await;
    let request = with_auth(
        Request::new(CreateRoleRequest {
            tenant_id: tenant_id.clone(),
            role_label: "Unauthorized Role".to_string(),
        }),
        &user_token,
    );

    let response = client.create_role(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::PermissionDenied);

    let _ = app.cleanup().await;
}

// ============================================================================
// Capability Assignment Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn user_with_specific_capability_can_access_endpoint() {
    let app = TestApp::spawn().await;
    let (tenant_id, root_org_id, superadmin_token, slug) = setup_with_superadmin(&app).await;

    // Create a regular user
    let mut auth_client = app.auth_client().await;
    let register_response = auth_client
        .register(Request::new(RegisterRequest {
            tenant_slug: slug.clone(),
            email: format!("specific@{}.com", slug),
            password: "Password123!".to_string(),
            display_name: None,
        }))
        .await
        .unwrap()
        .into_inner();
    let user_id = register_response.user.unwrap().user_id;

    // Create a role with org.node:read capability using superadmin
    let mut role_client = app.role_client().await;
    let create_role_response = role_client
        .create_role(with_auth(
            Request::new(CreateRoleRequest {
                tenant_id: tenant_id.clone(),
                role_label: "Org Reader".to_string(),
            }),
            &superadmin_token,
        ))
        .await
        .unwrap()
        .into_inner();
    let role_id = create_role_response.role.unwrap().role_id;

    // Assign org.node:read capability to the role
    use auth_service::grpc::proto::auth::AssignCapabilityRequest;
    role_client
        .assign_capability(with_auth(
            Request::new(AssignCapabilityRequest {
                role_id: role_id.clone(),
                capability_key: "org.node:read".to_string(),
            }),
            &superadmin_token,
        ))
        .await
        .unwrap();

    // Assign the role to the user
    let mut assignment_client = create_assignment_client(app.grpc_port).await;
    assignment_client
        .create_assignment(with_auth(
            Request::new(CreateAssignmentRequest {
                user_id: user_id.clone(),
                org_node_id: root_org_id.clone(),
                role_id: role_id.clone(),
                end_utc: None,
            }),
            &superadmin_token,
        ))
        .await
        .unwrap();

    // Now login as the user to get a fresh token with the assignment
    let login_response = auth_client
        .login(Request::new(LoginRequest {
            tenant_slug: slug.clone(),
            email: format!("specific@{}.com", slug),
            password: "Password123!".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    let user_token = login_response.access_token;

    // User should now be able to list org nodes (has org.node:read)
    let mut org_client = app.org_client().await;
    let request = with_auth(
        Request::new(ListTenantOrgNodesRequest {
            tenant_id: tenant_id.clone(),
        }),
        &user_token,
    );

    let response = org_client.list_tenant_org_nodes(request).await;
    assert!(
        response.is_ok(),
        "User with org.node:read should list org nodes: {:?}",
        response.err()
    );

    // But should NOT be able to create org nodes (doesn't have org.node:create)
    let request = with_auth(
        Request::new(CreateOrgNodeRequest {
            tenant_id: tenant_id.clone(),
            parent_org_node_id: Some(root_org_id.clone()),
            node_type_code: "team".to_string(),
            node_label: "Should Fail".to_string(),
        }),
        &user_token,
    );

    let response = org_client.create_org_node(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::PermissionDenied);

    let _ = app.cleanup().await;
}

// ============================================================================
// Public Endpoints Tests (No Capability Required)
// ============================================================================

#[tokio::test]
#[serial]
async fn auth_endpoints_are_public() {
    let app = TestApp::spawn().await;
    let (_, _, _, slug) = setup_with_superadmin(&app).await;
    let mut client = app.auth_client().await;

    // Register should work without auth
    let request = Request::new(RegisterRequest {
        tenant_slug: slug.clone(),
        email: format!("public@{}.com", slug),
        password: "Password123!".to_string(),
        display_name: None,
    });
    let response = client.register(request).await;
    assert!(
        response.is_ok(),
        "Register should be public: {:?}",
        response.err()
    );

    // Login should work without auth
    let request = Request::new(LoginRequest {
        tenant_slug: slug.clone(),
        email: format!("public@{}.com", slug),
        password: "Password123!".to_string(),
    });
    let response = client.login(request).await;
    assert!(
        response.is_ok(),
        "Login should be public: {:?}",
        response.err()
    );

    let _ = app.cleanup().await;
}
