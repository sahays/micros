//! Integration tests for delete_role, revoke_capability, and phone invitation flows.

use workflow_tests::helpers::auth::{with_admin_key, with_auth, TestApp};
use workflow_tests::proto::auth::{
    AcceptInvitationByPhoneRequest, AssignCapabilityRequest, BootstrapRequest,
    CreateInvitationRequest, CreateRoleRequest, DeleteRoleRequest, GetRoleRequest,
    RevokeCapabilityRequest,
};
use serial_test::serial;
use tonic::Request;
use uuid::Uuid;

/// Bootstrap and return (tenant_id, root_org_node_id, superadmin_role_id, admin_user_id, access_token).
async fn setup(app: &TestApp) -> (String, String, String, String, String) {
    let slug = format!("roletest-{}", &Uuid::new_v4().to_string()[..8]);
    let mut client = app.admin_client().await;
    let resp = client
        .bootstrap(with_admin_key(Request::new(BootstrapRequest {
            tenant_slug: slug.clone(),
            tenant_label: "Role Test Tenant".to_string(),
            admin_email: format!("admin@{}.com", slug),
            admin_password: "AdminPass123!".to_string(),
            admin_display_name: None,
        })))
        .await
        .unwrap()
        .into_inner();
    (
        resp.tenant_id,
        resp.root_org_node_id,
        resp.superadmin_role_id,
        resp.admin_user_id,
        resp.access_token,
    )
}

// ============================================================================
// Delete Role Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn delete_role_happy_path() {
    let app = TestApp::spawn().await;
    let (tenant_id, _, _, _, token) = setup(&app).await;

    let mut client = app.role_client().await;

    // Create a role
    let role = client
        .create_role(with_auth(
            Request::new(CreateRoleRequest {
                tenant_id: tenant_id.clone(),
                role_label: "Deletable Role".to_string(),
            }),
            &token,
        ))
        .await
        .unwrap()
        .into_inner()
        .role
        .unwrap();

    // Delete it
    let resp = client
        .delete_role(with_auth(
            Request::new(DeleteRoleRequest {
                role_id: role.role_id.clone(),
            }),
            &token,
        ))
        .await;
    assert!(resp.is_ok(), "Delete role should succeed: {:?}", resp.err());

    // Verify it's gone
    let get_resp = client
        .get_role(with_auth(
            Request::new(GetRoleRequest {
                role_id: role.role_id,
            }),
            &token,
        ))
        .await;
    assert!(get_resp.is_err());
    assert_eq!(get_resp.unwrap_err().code(), tonic::Code::NotFound);


}

#[tokio::test]
#[serial]
async fn delete_role_not_found() {
    let app = TestApp::spawn().await;
    let (_, _, _, _, token) = setup(&app).await;

    let mut client = app.role_client().await;

    let resp = client
        .delete_role(with_auth(
            Request::new(DeleteRoleRequest {
                role_id: uuid::Uuid::new_v4().to_string(),
            }),
            &token,
        ))
        .await;
    assert!(resp.is_err());
    assert_eq!(resp.unwrap_err().code(), tonic::Code::NotFound);


}

// ============================================================================
// Revoke Capability Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn revoke_capability_happy_path() {
    let app = TestApp::spawn().await;
    let (tenant_id, _, _, _, token) = setup(&app).await;

    let mut client = app.role_client().await;

    // Create a role and assign a capability
    let role = client
        .create_role(with_auth(
            Request::new(CreateRoleRequest {
                tenant_id: tenant_id.clone(),
                role_label: "Revoke Test Role".to_string(),
            }),
            &token,
        ))
        .await
        .unwrap()
        .into_inner()
        .role
        .unwrap();

    client
        .assign_capability(with_auth(
            Request::new(AssignCapabilityRequest {
                role_id: role.role_id.clone(),
                capability_key: "role:read".to_string(),
            }),
            &token,
        ))
        .await
        .unwrap();

    // Revoke it
    let resp = client
        .revoke_capability(with_auth(
            Request::new(RevokeCapabilityRequest {
                role_id: role.role_id.clone(),
                capability_key: "role:read".to_string(),
            }),
            &token,
        ))
        .await;
    assert!(
        resp.is_ok(),
        "Revoke capability should succeed: {:?}",
        resp.err()
    );


}

#[tokio::test]
#[serial]
async fn revoke_capability_not_found_role() {
    let app = TestApp::spawn().await;
    let (_, _, _, _, token) = setup(&app).await;

    let mut client = app.role_client().await;

    let resp = client
        .revoke_capability(with_auth(
            Request::new(RevokeCapabilityRequest {
                role_id: uuid::Uuid::new_v4().to_string(),
                capability_key: "role:read".to_string(),
            }),
            &token,
        ))
        .await;
    assert!(resp.is_err());
    assert_eq!(resp.unwrap_err().code(), tonic::Code::NotFound);


}

// ============================================================================
// Phone Invitation Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn phone_invitation_create_and_accept() {
    let app = TestApp::spawn().await;
    let (tenant_id, root_org_id, superadmin_role_id, admin_user_id, token) = setup(&app).await;

    let mut inv_client = app.invitation_client().await;

    // Create phone invitation
    let create_resp = inv_client
        .create_invitation(with_auth(
            Request::new(CreateInvitationRequest {
                tenant_id: tenant_id.clone(),
                email: String::new(),
                org_node_id: root_org_id.clone(),
                role_id: superadmin_role_id.clone(),
                inviter_user_id: admin_user_id.clone(), // placeholder
                expires_in_hours: Some(24),
                phone: Some("+1234567890".to_string()),
                verification_type: Some("phone".to_string()),
                metadata: Default::default(),
            }),
            &token,
        ))
        .await;
    assert!(
        create_resp.is_ok(),
        "Create phone invitation should succeed: {:?}",
        create_resp.err()
    );

    let create_inner = create_resp.unwrap().into_inner();
    let raw_token = create_inner.raw_token;
    let inv = create_inner.invitation.unwrap();
    assert_eq!(inv.verification_type, "phone");
    assert_eq!(inv.phone, "+1234567890");

    // Accept via phone
    let accept_resp = inv_client
        .accept_invitation_by_phone(Request::new(AcceptInvitationByPhoneRequest {
            token: raw_token,
            display_name: "Phone User".to_string(),
            password: Some("StrongPass123!".to_string()),
            profile_data: Default::default(),
        }))
        .await;
    assert!(
        accept_resp.is_ok(),
        "Accept phone invitation should succeed: {:?}",
        accept_resp.err()
    );

    let accept_inner = accept_resp.unwrap().into_inner();
    assert!(!accept_inner.auth.as_ref().unwrap().access_token.is_empty());
    assert_eq!(
        accept_inner
            .auth
            .as_ref()
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .display_name,
        Some("Phone User".to_string())
    );


}

#[tokio::test]
#[serial]
async fn phone_invitation_rejects_weak_password() {
    let app = TestApp::spawn().await;
    let (tenant_id, root_org_id, superadmin_role_id, admin_user_id, token) = setup(&app).await;

    let mut inv_client = app.invitation_client().await;

    // Create phone invitation
    let create_resp = inv_client
        .create_invitation(with_auth(
            Request::new(CreateInvitationRequest {
                tenant_id: tenant_id.clone(),
                email: String::new(),
                org_node_id: root_org_id.clone(),
                role_id: superadmin_role_id.clone(),
                inviter_user_id: admin_user_id.clone(),
                expires_in_hours: Some(24),
                phone: Some("+1234567891".to_string()),
                verification_type: Some("phone".to_string()),
                metadata: Default::default(),
            }),
            &token,
        ))
        .await
        .unwrap()
        .into_inner();

    // Accept with weak password
    let accept_resp = inv_client
        .accept_invitation_by_phone(Request::new(AcceptInvitationByPhoneRequest {
            token: create_resp.raw_token,
            display_name: "Weak Pass User".to_string(),
            password: Some("short".to_string()),
            profile_data: Default::default(),
        }))
        .await;
    assert!(accept_resp.is_err());
    assert_eq!(
        accept_resp.unwrap_err().code(),
        tonic::Code::InvalidArgument
    );


}
