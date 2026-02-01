//! Cross-Tenant Isolation Tests
//!
//! Verifies that data belonging to Tenant A is never accessible by Tenant B.
//! These tests create resources under one tenant and attempt to read them
//! from another tenant, expecting access to be denied or data to be invisible.

mod common;

use tonic::Request;
use uuid::Uuid;
use workflow_tests::proto::ledger::{
    AccountType, CreateAccountRequest, GetAccountRequest, ListAccountsRequest,
};
use workflow_tests::ServiceEndpoints;

/// Helper to create a gRPC request with tenant/user headers (BFF trust model).
fn with_tenant_headers<T>(inner: T, tenant_id: &str, user_id: &str) -> Request<T> {
    let mut request = Request::new(inner);
    request
        .metadata_mut()
        .insert("x-tenant-id", tenant_id.parse().unwrap());
    request
        .metadata_mut()
        .insert("x-user-id", user_id.parse().unwrap());
    request
}

/// Test: Tenant A's ledger account is not visible to Tenant B via GetAccount.
///
/// Creates an account under Tenant A, then attempts to retrieve it using
/// Tenant B's credentials. The account should not be found.
#[tokio::test]
async fn ledger_account_not_visible_to_other_tenant() {
    common::setup().await;

    let endpoints = ServiceEndpoints::from_env();
    let mut client = workflow_tests::LedgerServiceClient::connect(endpoints.ledger.clone())
        .await
        .expect("Failed to connect to ledger service");

    let tenant_a = Uuid::new_v4().to_string();
    let tenant_b = Uuid::new_v4().to_string();
    let user_a = Uuid::new_v4().to_string();
    let user_b = Uuid::new_v4().to_string();

    // Create account under Tenant A
    let create_req = with_tenant_headers(
        CreateAccountRequest {
            tenant_id: tenant_a.clone(),
            account_type: AccountType::Asset as i32,
            account_code: format!("ISO-{}", &Uuid::new_v4().to_string()[..8]),
            currency: "USD".to_string(),
            allow_negative: false,
            metadata: "{}".to_string(),
        },
        &tenant_a,
        &user_a,
    );

    let create_resp = client
        .create_account(create_req)
        .await
        .expect("Failed to create account for Tenant A");
    let account_id = create_resp
        .into_inner()
        .account
        .expect("Account should be present")
        .account_id;

    // Tenant A can see the account
    let get_req_a = with_tenant_headers(
        GetAccountRequest {
            tenant_id: tenant_a.clone(),
            account_id: account_id.clone(),
        },
        &tenant_a,
        &user_a,
    );
    let get_resp_a = client.get_account(get_req_a).await;
    assert!(get_resp_a.is_ok(), "Tenant A should see own account");

    // Tenant B cannot see the account
    let get_req_b = with_tenant_headers(
        GetAccountRequest {
            tenant_id: tenant_b.clone(),
            account_id: account_id.clone(),
        },
        &tenant_b,
        &user_b,
    );
    let get_resp_b = client.get_account(get_req_b).await;
    assert!(
        get_resp_b.is_err(),
        "Tenant B should NOT see Tenant A's account"
    );
}

/// Test: Tenant A's accounts don't appear in Tenant B's list.
///
/// Creates accounts under Tenant A, then lists accounts as Tenant B.
/// Tenant B's list should be empty (no cross-tenant leakage).
#[tokio::test]
async fn ledger_list_accounts_isolated_per_tenant() {
    common::setup().await;

    let endpoints = ServiceEndpoints::from_env();
    let mut client = workflow_tests::LedgerServiceClient::connect(endpoints.ledger.clone())
        .await
        .expect("Failed to connect to ledger service");

    let tenant_a = Uuid::new_v4().to_string();
    let tenant_b = Uuid::new_v4().to_string();
    let user_a = Uuid::new_v4().to_string();
    let user_b = Uuid::new_v4().to_string();

    // Create two accounts under Tenant A
    for i in 0..2 {
        let create_req = with_tenant_headers(
            CreateAccountRequest {
                tenant_id: tenant_a.clone(),
                account_type: AccountType::Asset as i32,
                account_code: format!("LIST-{}-{}", i, &Uuid::new_v4().to_string()[..8]),
                currency: "USD".to_string(),
                allow_negative: false,
                metadata: "{}".to_string(),
            },
            &tenant_a,
            &user_a,
        );
        client
            .create_account(create_req)
            .await
            .expect("Failed to create account");
    }

    // Tenant A sees 2 accounts
    let list_a = with_tenant_headers(
        ListAccountsRequest {
            tenant_id: tenant_a.clone(),
            account_type: 0,
            currency: String::new(),
            page_size: 100,
            page_token: String::new(),
        },
        &tenant_a,
        &user_a,
    );
    let resp_a = client
        .list_accounts(list_a)
        .await
        .expect("Tenant A list should succeed");
    assert!(
        resp_a.into_inner().accounts.len() >= 2,
        "Tenant A should see at least 2 accounts"
    );

    // Tenant B sees 0 accounts
    let list_b = with_tenant_headers(
        ListAccountsRequest {
            tenant_id: tenant_b.clone(),
            account_type: 0,
            currency: String::new(),
            page_size: 100,
            page_token: String::new(),
        },
        &tenant_b,
        &user_b,
    );
    let resp_b = client
        .list_accounts(list_b)
        .await
        .expect("Tenant B list should succeed");
    assert_eq!(
        resp_b.into_inner().accounts.len(),
        0,
        "Tenant B should see 0 accounts (no cross-tenant leakage)"
    );
}
