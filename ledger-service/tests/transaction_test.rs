//! Transaction Posting Integration Tests (Story 003)
//!
//! Run with: ./scripts/integ-tests.sh -p ledger-service

mod common;

use common::{create_test_account, get_balance, post_test_transaction, spawn_app};
use ledger_service::grpc::proto::{
    AccountType as ProtoAccountType, Direction as ProtoDirection, GetTransactionRequest,
    ListTransactionsRequest, PostTransactionEntry, PostTransactionRequest,
};
use uuid::Uuid;

/// Story 003, Task 1: Post valid two-entry transaction
#[tokio::test]
#[ignore]
async fn post_valid_two_entry_transaction() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create accounts
    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        false,
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

    // Post transaction: Debit Cash, Credit Revenue
    let response = post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        Some("2026-01-15"),
        None,
    )
    .await;

    let transaction = response.transaction.expect("Should return transaction");
    assert!(!transaction.journal_id.is_empty());
    assert_eq!(transaction.entries.len(), 2);
    assert_eq!(transaction.effective_date, "2026-01-15");
}

/// Story 003, Task 1: Reject transaction with unbalanced entries
#[tokio::test]
#[ignore]
async fn reject_unbalanced_transaction() {
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
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash.account.unwrap().account_id,
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: revenue.account.unwrap().account_id,
                amount: "90.00".to_string(), // Doesn't match!
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject unbalanced transaction");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("Double-entry violation"));
}

/// Story 003, Task 1: Reject transaction with single entry
#[tokio::test]
#[ignore]
async fn reject_single_entry_transaction() {
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

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![PostTransactionEntry {
            account_id: cash.account.unwrap().account_id,
            amount: "100.00".to_string(),
            direction: ProtoDirection::Debit as i32,
        }],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject single entry");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 003, Task 1: Reject transaction with zero amount
#[tokio::test]
#[ignore]
async fn reject_zero_amount_transaction() {
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
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash.account.unwrap().account_id,
                amount: "0".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: revenue.account.unwrap().account_id,
                amount: "0".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject zero amount");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 003, Task 1: Reject transaction with negative amount
#[tokio::test]
#[ignore]
async fn reject_negative_amount_transaction() {
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
    let revenue = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash.account.unwrap().account_id,
                amount: "-100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: revenue.account.unwrap().account_id,
                amount: "-100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject negative amount");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 003, Task 2: Reject cross-tenant account reference
#[tokio::test]
#[ignore]
async fn reject_cross_tenant_account_reference() {
    let (mut client, _) = spawn_app().await;

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create account for tenant1
    let cash = create_test_account(
        &mut client,
        tenant1,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        false,
    )
    .await;
    // Create account for tenant2
    let revenue = create_test_account(
        &mut client,
        tenant2,
        ProtoAccountType::Revenue,
        "REVENUE",
        "USD",
        false,
    )
    .await;

    // Try to post transaction using tenant1's transaction with tenant2's account
    let request = PostTransactionRequest {
        tenant_id: tenant1.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash.account.unwrap().account_id,
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: revenue.account.unwrap().account_id, // Wrong tenant!
                amount: "100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject cross-tenant reference");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status
        .message()
        .contains("does not exist or does not belong to tenant"));
}

/// Story 003, Task 2: Reject transaction with non-existent account
#[tokio::test]
#[ignore]
async fn reject_non_existent_account() {
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

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash.account.unwrap().account_id,
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: Uuid::new_v4().to_string(), // Non-existent!
                amount: "100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject non-existent account");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

/// Story 003, Task 2: Reject transaction with mismatched currencies
#[tokio::test]
#[ignore]
async fn reject_mismatched_currencies() {
    let (mut client, tenant_id) = spawn_app().await;

    let cash_usd = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-USD",
        "USD",
        false,
    )
    .await;
    let cash_eur = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH-EUR",
        "EUR",
        false,
    )
    .await;

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: cash_usd.account.unwrap().account_id,
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: cash_eur.account.unwrap().account_id,
                amount: "100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject currency mismatch");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("Currency mismatch"));
}

/// Story 003, Task 3: Idempotency key returns same transaction
#[tokio::test]
#[ignore]
async fn idempotency_key_returns_same_transaction() {
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

    let idempotency_key = Uuid::new_v4().to_string();

    // First request
    let response1 = post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        Some(&idempotency_key),
    )
    .await;
    let journal_id1 = response1.transaction.unwrap().journal_id;

    // Second request with same key - should return same transaction
    let response2 = post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        Some(&idempotency_key),
    )
    .await;
    let journal_id2 = response2.transaction.unwrap().journal_id;

    assert_eq!(journal_id1, journal_id2, "Should return same transaction");

    // Verify balance wasn't doubled
    let balance = get_balance(&mut client, tenant_id, &cash_id, None).await;
    assert_eq!(
        balance.balance, "100",
        "Balance should only reflect one transaction"
    );
}

/// Story 003, Task 4: Reject transaction that would make asset account negative
#[tokio::test]
#[ignore]
async fn reject_negative_balance_asset_account() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create asset account WITHOUT allow_negative
    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
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
    let expense_id = expense.account.unwrap().account_id;

    // Try to credit cash (reduce balance) when balance is 0
    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: expense_id.clone(),
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: cash_id.clone(),
                amount: "100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_err(), "Should reject negative balance");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("Insufficient balance"));
}

/// Story 003, Task 4: Allow negative balance when allow_negative is true
#[tokio::test]
#[ignore]
async fn allow_negative_balance_when_enabled() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create asset account WITH allow_negative
    let cash = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "CASH",
        "USD",
        true,
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
    let expense_id = expense.account.unwrap().account_id;

    // This should succeed because allow_negative is true
    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: expense_id.clone(),
                amount: "100.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: cash_id.clone(),
                amount: "100.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };

    let result = client.post_transaction(request).await;
    assert!(result.is_ok(), "Should allow negative balance when enabled");
}

/// Story 003, Task 5: Get transaction by journal ID
#[tokio::test]
#[ignore]
async fn get_transaction_by_journal_id() {
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

    let posted = post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        None,
    )
    .await;
    let journal_id = posted.transaction.unwrap().journal_id;

    // Get the transaction
    let request = GetTransactionRequest {
        tenant_id: tenant_id.to_string(),
        journal_id: journal_id.clone(),
    };

    let response = client.get_transaction(request).await.unwrap().into_inner();
    let transaction = response.transaction.expect("Should return transaction");

    assert_eq!(transaction.journal_id, journal_id);
    assert_eq!(transaction.entries.len(), 2);
}

/// Story 003, Task 5: Get transaction wrong tenant returns not found
#[tokio::test]
#[ignore]
async fn get_transaction_wrong_tenant_not_found() {
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

    let posted = post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        None,
    )
    .await;
    let journal_id = posted.transaction.unwrap().journal_id;

    // Try with wrong tenant
    let request = GetTransactionRequest {
        tenant_id: Uuid::new_v4().to_string(),
        journal_id,
    };

    let result = client.get_transaction(request).await;
    assert!(result.is_err(), "Should return not found for wrong tenant");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

/// Story 003, Task 6: List transactions with filters
#[tokio::test]
#[ignore]
async fn list_transactions_with_filters() {
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

    // Post multiple transactions
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
        &expense_id,
        &cash_id,
        "50.00",
        Some("2026-01-20"),
        None,
    )
    .await;

    // List all transactions
    let request = ListTransactionsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_id: String::new(),
        start_date: String::new(),
        end_date: String::new(),
    };

    let response = client
        .list_transactions(request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.transactions.len(), 3);

    // Filter by account
    let request = ListTransactionsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_id: cash_id.clone(),
        start_date: String::new(),
        end_date: String::new(),
    };

    let response = client
        .list_transactions(request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        response.transactions.len(),
        3,
        "All transactions involve cash"
    );

    // Filter by date range
    let request = ListTransactionsRequest {
        tenant_id: tenant_id.to_string(),
        page_size: 10,
        page_token: String::new(),
        account_id: String::new(),
        start_date: "2026-01-12".to_string(),
        end_date: "2026-01-18".to_string(),
    };

    let response = client
        .list_transactions(request)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        response.transactions.len(),
        1,
        "Only one transaction in range"
    );
}
