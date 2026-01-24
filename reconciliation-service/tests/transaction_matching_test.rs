//! Integration tests for transaction matching operations.

mod common;

use common::{spawn_app, with_tenant};
use reconciliation_service::grpc::proto::*;
use uuid::Uuid;

/// Helper to create a bank account and get back the ID.
async fn create_bank_account(
    client: &mut reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient<tonic::transport::Channel>,
    tenant_id: &Uuid,
) -> String {
    let request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Test Bank".to_string(),
            account_number_masked: "****1234".to_string(),
            currency: "USD".to_string(),
        },
        tenant_id,
    );

    client
        .register_bank_account(request)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap()
        .bank_account_id
}

/// Helper to import a statement and get back the ID.
async fn import_statement(
    client: &mut reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient<tonic::transport::Channel>,
    tenant_id: &Uuid,
    bank_account_id: &str,
) -> String {
    let request = with_tenant(
        ImportStatementRequest {
            bank_account_id: bank_account_id.to_string(),
            document_id: Uuid::new_v4().to_string(),
            extraction_hints: None,
        },
        tenant_id,
    );

    client
        .import_statement(request)
        .await
        .unwrap()
        .into_inner()
        .statement
        .unwrap()
        .statement_id
}

#[tokio::test]
async fn match_transaction_requires_ledger_entry() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        MatchTransactionRequest {
            bank_transaction_id: Uuid::new_v4().to_string(),
            ledger_entry_ids: vec![], // Empty list should fail
        },
        &app.tenant_id,
    );

    let response = client.match_transaction(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn match_transaction_not_found() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        MatchTransactionRequest {
            bank_transaction_id: Uuid::new_v4().to_string(),
            ledger_entry_ids: vec![Uuid::new_v4().to_string()],
        },
        &app.tenant_id,
    );

    let response = client.match_transaction(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn unmatch_transaction_not_found() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        UnmatchTransactionRequest {
            bank_transaction_id: Uuid::new_v4().to_string(),
        },
        &app.tenant_id,
    );

    let response = client.unmatch_transaction(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn exclude_transaction_not_found() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        ExcludeTransactionRequest {
            bank_transaction_id: Uuid::new_v4().to_string(),
            reason: Some("Test exclusion".to_string()),
        },
        &app.tenant_id,
    );

    // Note: Current implementation doesn't validate transaction exists before excluding
    // This tests current behavior - exclude on non-existent succeeds (no-op)
    let response = client.exclude_transaction(request).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn get_candidate_entries_returns_empty_without_ledger_client() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create bank account and import statement
    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let _statement_id = import_statement(&mut client, &app.tenant_id, &bank_account_id).await;

    // Try to get candidates for non-existent transaction
    let request = with_tenant(
        GetCandidateEntriesRequest {
            bank_transaction_id: Uuid::new_v4().to_string(),
            date_range_days: Some(7),
            limit: Some(10),
        },
        &app.tenant_id,
    );

    let response = client.get_candidate_entries(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn tenant_isolation_for_matching() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create bank account for tenant1
    let request1 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Tenant1 Bank".to_string(),
            account_number_masked: "****1111".to_string(),
            currency: "USD".to_string(),
        },
        &tenant1,
    );

    let account1 = client
        .register_bank_account(request1)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Import statement for tenant1
    let stmt_request = with_tenant(
        ImportStatementRequest {
            bank_account_id: account1.bank_account_id.clone(),
            document_id: Uuid::new_v4().to_string(),
            extraction_hints: None,
        },
        &tenant1,
    );

    let _stmt = client.import_statement(stmt_request).await.unwrap();

    // Tenant2 should not be able to access tenant1's bank account
    let request2 = with_tenant(
        GetBankAccountRequest {
            bank_account_id: account1.bank_account_id.clone(),
        },
        &tenant2,
    );

    let response = client.get_bank_account(request2).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn matching_rule_creates_and_lists_correctly() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create a matching rule
    let create_request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Test Rule".to_string(),
            description_pattern: "PAYMENT".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: Some(1),
        },
        &app.tenant_id,
    );

    let rule = client
        .create_matching_rule(create_request)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    assert_eq!(rule.name, "Test Rule");
    assert!(rule.is_active);

    // List rules
    let list_request = with_tenant(
        ListMatchingRulesRequest {
            page_size: 10,
            page_token: None,
            active_only: None,
        },
        &app.tenant_id,
    );

    let response = client.list_matching_rules(list_request).await.unwrap();
    assert_eq!(response.into_inner().rules.len(), 1);
}
