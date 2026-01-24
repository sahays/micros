//! Integration tests for bank account operations.

mod common;

use common::{spawn_app, with_tenant};
use reconciliation_service::grpc::proto::*;
use uuid::Uuid;

#[tokio::test]
async fn register_bank_account_creates_account() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let ledger_account_id = Uuid::new_v4();
    let request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: ledger_account_id.to_string(),
            bank_name: "Test Bank".to_string(),
            account_number_masked: "1234".to_string(),
            currency: "USD".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.register_bank_account(request).await;
    assert!(response.is_ok(), "Expected OK, got: {:?}", response);

    let bank_account = response.unwrap().into_inner().bank_account.unwrap();
    assert_eq!(bank_account.bank_name, "Test Bank");
    assert_eq!(bank_account.account_number_masked, "1234");
    assert_eq!(bank_account.currency, "USD");
    assert_eq!(
        bank_account.ledger_account_id,
        ledger_account_id.to_string()
    );
    assert!(bank_account.last_reconciled_date.is_none());
    assert!(bank_account.last_reconciled_balance.is_none());
}

#[tokio::test]
async fn get_bank_account_returns_account() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // First register an account
    let ledger_account_id = Uuid::new_v4();
    let register_request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: ledger_account_id.to_string(),
            bank_name: "Test Bank".to_string(),
            account_number_masked: "5678".to_string(),
            currency: "EUR".to_string(),
        },
        &app.tenant_id,
    );

    let register_response = client
        .register_bank_account(register_request)
        .await
        .unwrap();
    let bank_account_id = register_response
        .into_inner()
        .bank_account
        .unwrap()
        .bank_account_id;

    // Now get it
    let get_request = with_tenant(
        GetBankAccountRequest {
            bank_account_id: bank_account_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.get_bank_account(get_request).await;
    assert!(response.is_ok());

    let bank_account = response.unwrap().into_inner().bank_account.unwrap();
    assert_eq!(bank_account.bank_account_id, bank_account_id);
    assert_eq!(bank_account.bank_name, "Test Bank");
}

#[tokio::test]
async fn get_bank_account_not_found_returns_error() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        GetBankAccountRequest {
            bank_account_id: Uuid::new_v4().to_string(),
        },
        &app.tenant_id,
    );

    let response = client.get_bank_account(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn list_bank_accounts_returns_only_tenant_accounts() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create accounts for this tenant
    for i in 0..3 {
        let request = with_tenant(
            RegisterBankAccountRequest {
                ledger_account_id: Uuid::new_v4().to_string(),
                bank_name: format!("Bank {}", i),
                account_number_masked: format!("{:04}", i),
                currency: "USD".to_string(),
            },
            &app.tenant_id,
        );
        client.register_bank_account(request).await.unwrap();
    }

    // List accounts
    let list_request = with_tenant(
        ListBankAccountsRequest {
            page_size: 10,
            page_token: None,
        },
        &app.tenant_id,
    );

    let response = client.list_bank_accounts(list_request).await;
    assert!(response.is_ok());

    let list_response = response.unwrap().into_inner();
    assert_eq!(list_response.bank_accounts.len(), 3);
}

#[tokio::test]
async fn list_bank_accounts_pagination_works() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create 5 accounts
    for i in 0..5 {
        let request = with_tenant(
            RegisterBankAccountRequest {
                ledger_account_id: Uuid::new_v4().to_string(),
                bank_name: format!("Bank {}", i),
                account_number_masked: format!("{:04}", i),
                currency: "USD".to_string(),
            },
            &app.tenant_id,
        );
        client.register_bank_account(request).await.unwrap();
    }

    // Get first page
    let list_request = with_tenant(
        ListBankAccountsRequest {
            page_size: 2,
            page_token: None,
        },
        &app.tenant_id,
    );

    let response = client
        .list_bank_accounts(list_request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.bank_accounts.len(), 2);
    assert!(response.next_page_token.is_some());

    // Get second page
    let list_request = with_tenant(
        ListBankAccountsRequest {
            page_size: 2,
            page_token: response.next_page_token,
        },
        &app.tenant_id,
    );

    let response = client
        .list_bank_accounts(list_request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.bank_accounts.len(), 2);
    assert!(response.next_page_token.is_some());

    // Get third page (should have 1 item)
    let list_request = with_tenant(
        ListBankAccountsRequest {
            page_size: 2,
            page_token: response.next_page_token,
        },
        &app.tenant_id,
    );

    let response = client
        .list_bank_accounts(list_request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.bank_accounts.len(), 1);
    assert!(response.next_page_token.is_none());
}

#[tokio::test]
async fn update_bank_account_updates_mutable_fields() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create account
    let ledger_account_id = Uuid::new_v4();
    let register_request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: ledger_account_id.to_string(),
            bank_name: "Original Bank".to_string(),
            account_number_masked: "1111".to_string(),
            currency: "USD".to_string(),
        },
        &app.tenant_id,
    );

    let bank_account = client
        .register_bank_account(register_request)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Update it
    let update_request = with_tenant(
        UpdateBankAccountRequest {
            bank_account_id: bank_account.bank_account_id.clone(),
            bank_name: Some("Updated Bank".to_string()),
            account_number_masked: Some("9999".to_string()),
        },
        &app.tenant_id,
    );

    let response = client.update_bank_account(update_request).await;
    assert!(response.is_ok());

    let updated = response.unwrap().into_inner().bank_account.unwrap();
    assert_eq!(updated.bank_name, "Updated Bank");
    assert_eq!(updated.account_number_masked, "9999");
    // These should not change
    assert_eq!(updated.currency, "USD");
    assert_eq!(updated.ledger_account_id, ledger_account_id.to_string());
}

#[tokio::test]
async fn update_bank_account_partial_update_works() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create account
    let register_request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Original Bank".to_string(),
            account_number_masked: "1111".to_string(),
            currency: "USD".to_string(),
        },
        &app.tenant_id,
    );

    let bank_account = client
        .register_bank_account(register_request)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Update only bank name
    let update_request = with_tenant(
        UpdateBankAccountRequest {
            bank_account_id: bank_account.bank_account_id.clone(),
            bank_name: Some("New Bank Name".to_string()),
            account_number_masked: None,
        },
        &app.tenant_id,
    );

    let response = client.update_bank_account(update_request).await;
    assert!(response.is_ok());

    let updated = response.unwrap().into_inner().bank_account.unwrap();
    assert_eq!(updated.bank_name, "New Bank Name");
    assert_eq!(updated.account_number_masked, "1111"); // Unchanged
}

#[tokio::test]
async fn update_bank_account_not_found_returns_error() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let update_request = with_tenant(
        UpdateBankAccountRequest {
            bank_account_id: Uuid::new_v4().to_string(),
            bank_name: Some("New Name".to_string()),
            account_number_masked: None,
        },
        &app.tenant_id,
    );

    let response = client.update_bank_account(update_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn register_duplicate_ledger_account_id_returns_error() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let ledger_account_id = Uuid::new_v4();

    // First registration should succeed
    let request1 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: ledger_account_id.to_string(),
            bank_name: "First Bank".to_string(),
            account_number_masked: "1111".to_string(),
            currency: "USD".to_string(),
        },
        &app.tenant_id,
    );

    let response1 = client.register_bank_account(request1).await;
    assert!(response1.is_ok());

    // Second registration with same ledger_account_id should fail
    let request2 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: ledger_account_id.to_string(),
            bank_name: "Second Bank".to_string(),
            account_number_masked: "2222".to_string(),
            currency: "EUR".to_string(),
        },
        &app.tenant_id,
    );

    let response2 = client.register_bank_account(request2).await;
    assert!(response2.is_err());
    assert_eq!(response2.unwrap_err().code(), tonic::Code::AlreadyExists);
}

#[tokio::test]
async fn tenant_isolation_works() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create account for tenant1
    let request1 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Tenant1 Bank".to_string(),
            account_number_masked: "1111".to_string(),
            currency: "USD".to_string(),
        },
        &tenant1,
    );

    let bank_account1 = client
        .register_bank_account(request1)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Create account for tenant2
    let request2 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Tenant2 Bank".to_string(),
            account_number_masked: "2222".to_string(),
            currency: "EUR".to_string(),
        },
        &tenant2,
    );

    client.register_bank_account(request2).await.unwrap();

    // Tenant1 should not see tenant2's account
    let list_request1 = with_tenant(
        ListBankAccountsRequest {
            page_size: 10,
            page_token: None,
        },
        &tenant1,
    );

    let list1 = client
        .list_bank_accounts(list_request1)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list1.bank_accounts.len(), 1);
    assert_eq!(list1.bank_accounts[0].bank_name, "Tenant1 Bank");

    // Tenant2 trying to get tenant1's account should fail
    let get_request = with_tenant(
        GetBankAccountRequest {
            bank_account_id: bank_account1.bank_account_id,
        },
        &tenant2,
    );

    let response = client.get_bank_account(get_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}
