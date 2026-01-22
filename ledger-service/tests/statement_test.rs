//! Account Statement Integration Tests (Story 005)
//!
//! Run with: ./scripts/integ-tests.sh -p ledger-service

mod common;

use common::{create_test_account, post_test_transaction, spawn_app};
use ledger_service::grpc::proto::{
    AccountType as ProtoAccountType, Direction as ProtoDirection, GetStatementRequest,
    PostTransactionEntry, PostTransactionRequest,
};

/// Story 005, Task 1: Get statement with entries
#[tokio::test]
async fn get_statement_with_entries() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
    )
    .await;
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;

    // Post transactions
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        Some("2026-01-10"),
        None,
    )
    .await;
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "200.00",
        Some("2026-01-15"),
        None,
    )
    .await;
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "50.00",
        Some("2026-01-20"),
        None,
    )
    .await;

    // Get statement for Jan 1-31
    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: cash_id.clone(),
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
    };

    let response = client.get_statement(request).await.unwrap().into_inner();

    assert_eq!(response.account_id, cash_id);
    assert_eq!(response.currency, "USD");
    assert_eq!(response.opening_balance, "0"); // No prior transactions
    assert_eq!(response.closing_balance, "350"); // 100 + 200 + 50
    assert_eq!(response.lines.len(), 3);

    // Verify entries are ordered by date
    assert_eq!(response.lines[0].effective_date, "2026-01-10");
    assert_eq!(response.lines[1].effective_date, "2026-01-15");
    assert_eq!(response.lines[2].effective_date, "2026-01-20");
}

/// Story 005, Task 2: Statement with opening balance from prior period
#[tokio::test]
async fn get_statement_with_opening_balance() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
    )
    .await;
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;

    // Post transaction BEFORE the statement period
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "500.00",
        Some("2026-01-05"),
        None,
    )
    .await;

    // Post transactions WITHIN the statement period
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        Some("2026-01-15"),
        None,
    )
    .await;
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "200.00",
        Some("2026-01-20"),
        None,
    )
    .await;

    // Get statement for Jan 10-31 (excludes the Jan 5 transaction)
    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: cash_id.clone(),
        start_date: "2026-01-10".to_string(),
        end_date: "2026-01-31".to_string(),
    };

    let response = client.get_statement(request).await.unwrap().into_inner();

    assert_eq!(
        response.opening_balance, "500",
        "Opening should include Jan 5 transaction"
    );
    assert_eq!(response.closing_balance, "800"); // 500 + 100 + 200
    assert_eq!(response.lines.len(), 2); // Only Jan 15 and Jan 20 transactions
}

/// Story 005, Task 3: Statement with running balance
#[tokio::test]
async fn get_statement_with_running_balance() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
    )
    .await;
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;
    let expense = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Expense,
        "EXPENSE",
        "USD",
        true,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;
    let expense_id = expense.account.unwrap().account_id;

    // Debit cash (increase)
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "1000.00",
        Some("2026-01-10"),
        None,
    )
    .await;

    // Credit cash (decrease)
    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: expense_id.clone(),
                amount: "300.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: cash_id.clone(),
                amount: "300.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: "2026-01-15".to_string(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };
    client.post_transaction(request).await.unwrap();

    // Debit cash again
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "500.00",
        Some("2026-01-20"),
        None,
    )
    .await;

    // Get statement
    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: cash_id.clone(),
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
    };

    let response = client.get_statement(request).await.unwrap().into_inner();

    assert_eq!(response.opening_balance, "0");
    assert_eq!(response.lines.len(), 3);

    // Running balances should be: 1000, 700, 1200
    assert_eq!(response.lines[0].running_balance, "1000");
    assert_eq!(response.lines[1].running_balance, "700"); // 1000 - 300
    assert_eq!(response.lines[2].running_balance, "1200"); // 700 + 500

    assert_eq!(response.closing_balance, "1200");
}

/// Story 005, Task 4: Empty statement (no transactions in period)
#[tokio::test]
async fn get_statement_empty_period() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
    )
    .await;
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;

    // Post transaction in January
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        Some("2026-01-15"),
        None,
    )
    .await;

    // Get statement for February (no transactions)
    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: cash_id.clone(),
        start_date: "2026-02-01".to_string(),
        end_date: "2026-02-28".to_string(),
    };

    let response = client.get_statement(request).await.unwrap().into_inner();

    assert_eq!(response.opening_balance, "100"); // From January transaction
    assert_eq!(response.closing_balance, "100");
    assert_eq!(response.lines.len(), 0);
}

/// Story 005, Task 4: Statement not found for non-existent account
#[tokio::test]
async fn get_statement_account_not_found() {
    let (mut client, tenant_id) = spawn_app().await;

    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: uuid::Uuid::new_v4().to_string(),
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
    };

    let result = client.get_statement(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

/// Story 005, Task 4: Reject invalid date range (end before start)
#[tokio::test]
async fn get_statement_invalid_date_range() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        false,
    )
    .await;
    let cash_id = cash.account.unwrap().account_id;

    let request = GetStatementRequest {
        tenant_id: tenant_id.to_string(),
        account_id: cash_id,
        start_date: "2026-01-31".to_string(),
        end_date: "2026-01-01".to_string(), // Before start!
    };

    let result = client.get_statement(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 005: Statement tenant isolation
#[tokio::test]
async fn get_statement_tenant_isolation() {
    let (mut client, _) = spawn_app().await;

    let tenant1 = uuid::Uuid::new_v4();
    let tenant2 = uuid::Uuid::new_v4();

    // Create account for tenant1
    let cash = create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
    )
    .await;
    let revenue = create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;

    // Post transaction for tenant1
    post_test_transaction(
        &mut client,
        tenant1,
        &cash_id,
        &revenue_id,
        "100.00",
        Some("2026-01-15"),
        None,
    )
    .await;

    // Try to get statement using tenant2
    let request = GetStatementRequest {
        tenant_id: tenant2.to_string(),
        account_id: cash_id,
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
    };

    let result = client.get_statement(request).await;
    assert!(
        result.is_err(),
        "Should not find account from different tenant"
    );
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}
