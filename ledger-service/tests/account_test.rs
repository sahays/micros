//! Account Management Integration Tests (Story 002)
//!
//! Run with: ./scripts/integ-tests.sh -p ledger-service

mod common;

use common::{create_test_account, spawn_app};
use ledger_service::grpc::proto::{
    AccountType as ProtoAccountType, GetAccountRequest, ListAccountsRequest,
};
use uuid::Uuid;

/// Story 002, Task 1: Create account with valid parameters
#[tokio::test]
#[ignore] // Requires database - run with integ-tests.sh
async fn create_account_with_valid_parameters() {
    let (mut client, tenant_id) = spawn_app().await;

    let response = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;

    let account = response.account.expect("Should return account");
    assert_eq!(account.tenant_id, tenant_id.to_string());
    assert_eq!(account.account_code, "CASH-001");
    assert_eq!(account.currency, "USD");
    assert_eq!(account.account_type, ProtoAccountType::Asset as i32);
    assert!(!account.allow_negative);
    assert_eq!(account.balance, "0"); // New accounts have zero balance
}

/// Story 002, Task 1: Create account with all account types
#[tokio::test]
#[ignore]
async fn create_account_with_all_types() {
    let (mut client, tenant_id) = spawn_app().await;

    // Asset
    let resp = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "ASSET-001",
        "USD",
        false,
    )
    .await;
    assert_eq!(
        resp.account.unwrap().account_type,
        ProtoAccountType::Asset as i32
    );

    // Liability
    let resp = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Liability,
        "LIAB-001",
        "USD",
        false,
    )
    .await;
    assert_eq!(
        resp.account.unwrap().account_type,
        ProtoAccountType::Liability as i32
    );

    // Equity
    let resp = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Equity,
        "EQUITY-001",
        "USD",
        false,
    )
    .await;
    assert_eq!(
        resp.account.unwrap().account_type,
        ProtoAccountType::Equity as i32
    );

    // Revenue
    let resp = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REV-001",
        "USD",
        false,
    )
    .await;
    assert_eq!(
        resp.account.unwrap().account_type,
        ProtoAccountType::Revenue as i32
    );

    // Expense
    let resp = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Expense,
        "EXP-001",
        "USD",
        false,
    )
    .await;
    assert_eq!(
        resp.account.unwrap().account_type,
        ProtoAccountType::Expense as i32
    );
}

/// Story 002, Task 1: Reject duplicate account code within same tenant
#[tokio::test]
#[ignore]
async fn reject_duplicate_account_code_same_tenant() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create first account
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;

    // Try to create duplicate
    let request = ledger_service::grpc::proto::CreateAccountRequest {
        tenant_id: tenant_id.to_string(),
        account_type: ProtoAccountType::Asset as i32,
        account_code: "CASH-001".to_string(),
        currency: "USD".to_string(),
        allow_negative: false,
        metadata: String::new(),
    };

    let result = client.create_account(request).await;
    assert!(result.is_err(), "Should reject duplicate account code");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::AlreadyExists);
}

/// Story 002, Task 1: Allow same account code in different tenants
#[tokio::test]
#[ignore]
async fn allow_same_account_code_different_tenant() {
    let (mut client, _tenant1) = spawn_app().await;

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create account for tenant1
    create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;

    // Create same account code for tenant2 - should succeed
    let resp = create_test_account(
        &mut client,
        tenant2,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;
    let account = resp
        .account
        .expect("Should create account for different tenant");
    assert_eq!(account.tenant_id, tenant2.to_string());
}

/// Story 002, Task 1: Reject invalid currency code
#[tokio::test]
#[ignore]
async fn reject_invalid_currency_code() {
    let (mut client, tenant_id) = spawn_app().await;

    let request = ledger_service::grpc::proto::CreateAccountRequest {
        tenant_id: tenant_id.to_string(),
        account_type: ProtoAccountType::Asset as i32,
        account_code: "CASH-001".to_string(),
        currency: "INVALID".to_string(), // Not 3 characters
        allow_negative: false,
        metadata: String::new(),
    };

    let result = client.create_account(request).await;
    assert!(result.is_err(), "Should reject invalid currency");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 002, Task 2: Get account by ID
#[tokio::test]
#[ignore]
async fn get_account_by_id() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create account
    let created = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;
    let created_account = created.account.unwrap();

    // Get account
    let request = GetAccountRequest {
        tenant_id: tenant_id.to_string(),
        account_id: created_account.account_id.clone(),
    };

    let response = client.get_account(request).await.unwrap().into_inner();
    let account = response.account.expect("Should return account");

    assert_eq!(account.account_id, created_account.account_id);
    assert_eq!(account.account_code, "CASH-001");
    assert_eq!(account.balance, "0"); // Should include balance
}

/// Story 002, Task 2: Get account returns not found for wrong tenant
#[tokio::test]
#[ignore]
async fn get_account_wrong_tenant_returns_not_found() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create account
    let created = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;
    let created_account = created.account.unwrap();

    // Try to get with different tenant
    let wrong_tenant = Uuid::new_v4();
    let request = GetAccountRequest {
        tenant_id: wrong_tenant.to_string(),
        account_id: created_account.account_id,
    };

    let result = client.get_account(request).await;
    assert!(result.is_err(), "Should return error for wrong tenant");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

/// Story 002, Task 3: List accounts with pagination
#[tokio::test]
#[ignore]
async fn list_accounts_with_pagination() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create 5 accounts
    for i in 1..=5 {
        create_test_account(
            &mut client,
            tenant_id,
            ProtoAccountType::Asset,
            &format!("CASH-{:03}", i),
            "USD",
            false,
        )
        .await;
    }

    // List with page size 2
    let request = ListAccountsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 2,
        page_token: String::new(),
        account_type: 0,
        currency: String::new(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(response.accounts.len(), 2);
    assert!(
        !response.next_page_token.is_empty(),
        "Should have next page"
    );

    // Get next page
    let request = ListAccountsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 2,
        page_token: response.next_page_token,
        account_type: 0,
        currency: String::new(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(response.accounts.len(), 2);
    assert!(!response.next_page_token.is_empty());

    // Get last page
    let request = ListAccountsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 2,
        page_token: response.next_page_token,
        account_type: 0,
        currency: String::new(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(response.accounts.len(), 1); // Only 1 left
    assert!(response.next_page_token.is_empty(), "Should be last page");
}

/// Story 002, Task 3: Filter accounts by type
#[tokio::test]
#[ignore]
async fn list_accounts_filter_by_type() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create different types
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Liability,
        "LOAN-001",
        "USD",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-002",
        "USD",
        false,
    )
    .await;

    // List only assets
    let request = ListAccountsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_type: ProtoAccountType::Asset as i32,
        currency: String::new(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(response.accounts.len(), 2, "Should only return assets");
    for account in response.accounts {
        assert_eq!(account.account_type, ProtoAccountType::Asset as i32);
    }
}

/// Story 002, Task 3: Filter accounts by currency
#[tokio::test]
#[ignore]
async fn list_accounts_filter_by_currency() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create different currencies
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-USD",
        "USD",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-EUR",
        "EUR",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-USD2",
        "USD",
        false,
    )
    .await;

    // List only USD
    let request = ListAccountsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_type: 0,
        currency: "USD".to_string(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(
        response.accounts.len(),
        2,
        "Should only return USD accounts"
    );
    for account in response.accounts {
        assert_eq!(account.currency, "USD");
    }
}

/// Story 002, Task 3: Tenant isolation in list accounts
#[tokio::test]
#[ignore]
async fn list_accounts_tenant_isolation() {
    let (mut client, _) = spawn_app().await;

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create accounts for different tenants
    create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Asset,
        "CASH-002",
        "USD",
        false,
    )
    .await;
    create_test_account(
        &mut client,
        tenant2,
        ProtoAccountType::Asset,
        "CASH-001",
        "USD",
        false,
    )
    .await;

    // List tenant1 accounts
    let request = ListAccountsRequest {
        tenant_id: tenant1.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_type: 0,
        currency: String::new(),
    };

    let response = client.list_accounts(request).await.unwrap().into_inner();
    assert_eq!(
        response.accounts.len(),
        2,
        "Should only return tenant1 accounts"
    );
    for account in response.accounts {
        assert_eq!(account.tenant_id, tenant1.to_string());
    }
}
