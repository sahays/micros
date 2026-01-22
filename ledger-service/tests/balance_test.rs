//! Balance Query Integration Tests (Story 004)
//!
//! Run with: ./scripts/integ-tests.sh -p ledger-service

mod common;

use common::{create_test_account, get_balance, post_test_transaction, spawn_app};
use ledger_service::grpc::proto::{
    AccountType as ProtoAccountType, Direction as ProtoDirection, GetBalancesRequest,
    PostTransactionEntry, PostTransactionRequest,
};
use uuid::Uuid;

/// Story 004, Task 1: Get balance for asset account (debit-normal)
#[tokio::test]
async fn get_balance_asset_account() {
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

    // Debit cash (increase), credit revenue
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        None,
    )
    .await;

    let balance = get_balance(&mut client, tenant_id, &cash_id, None).await;
    assert_eq!(
        balance.balance, "100",
        "Asset debit should increase balance"
    );
    assert_eq!(balance.currency, "USD");
}

/// Story 004, Task 1: Get balance for liability account (credit-normal)
#[tokio::test]
async fn get_balance_liability_account() {
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
    let loan = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Liability,
        "LOAN",
        "USD",
        true,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let loan_id = loan.account.unwrap().account_id;

    // Debit cash, credit loan (liability increases with credit)
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &loan_id,
        "100.00",
        None,
        None,
    )
    .await;

    let balance = get_balance(&mut client, tenant_id, &loan_id, None).await;
    assert_eq!(
        balance.balance, "100",
        "Liability credit should increase balance"
    );
}

/// Story 004, Task 1: Get balance for revenue account (credit-normal)
#[tokio::test]
async fn get_balance_revenue_account() {
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
        true,
    )
    .await;

    let cash_id = cash.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;

    // Debit cash, credit revenue
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        None,
    )
    .await;

    let balance = get_balance(&mut client, tenant_id, &revenue_id, None).await;
    assert_eq!(
        balance.balance, "100",
        "Revenue credit should increase balance"
    );
}

/// Story 004, Task 1: Get balance for expense account (debit-normal)
#[tokio::test]
async fn get_balance_expense_account() {
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

    // First fund the cash account
    let equity = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Equity,
        "EQUITY",
        "USD",
        true,
    )
    .await;
    let equity_id = equity.account.unwrap().account_id;
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &equity_id,
        "500.00",
        None,
        None,
    )
    .await;

    // Debit expense (increase), credit cash
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
    client.post_transaction(request).await.unwrap();

    let balance = get_balance(&mut client, tenant_id, &expense_id, None).await;
    assert_eq!(
        balance.balance, "100",
        "Expense debit should increase balance"
    );
}

/// Story 004, Task 2: Get balance as of historical date
#[tokio::test]
async fn get_balance_as_of_date() {
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

    // Post transactions on different dates
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

    // Balance as of Jan 12 should be 100
    let balance = get_balance(&mut client, tenant_id, &cash_id, Some("2026-01-12")).await;
    assert_eq!(balance.balance, "100");

    // Balance as of Jan 16 should be 300
    let balance = get_balance(&mut client, tenant_id, &cash_id, Some("2026-01-16")).await;
    assert_eq!(balance.balance, "300");

    // Balance as of Jan 25 should be 350
    let balance = get_balance(&mut client, tenant_id, &cash_id, Some("2026-01-25")).await;
    assert_eq!(balance.balance, "350");
}

/// Story 004, Task 2: Balance of zero for account with no transactions
#[tokio::test]
async fn get_balance_no_transactions() {
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

    let balance = get_balance(&mut client, tenant_id, &cash_id, None).await;
    assert_eq!(balance.balance, "0", "New account should have zero balance");
}

/// Story 004, Task 2: Balance not found for non-existent account
#[tokio::test]
async fn get_balance_not_found() {
    let (mut client, tenant_id) = spawn_app().await;

    let request = ledger_service::grpc::proto::GetBalanceRequest {
        tenant_id: tenant_id.to_string(),
        account_id: Uuid::new_v4().to_string(),
        as_of_date: String::new(),
    };

    let result = client.get_balance(request).await;
    assert!(
        result.is_err(),
        "Should return error for non-existent account"
    );
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

/// Story 004, Task 3: Get balances for multiple accounts
#[tokio::test]
async fn get_balances_multiple_accounts() {
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

    // Post transactions
    post_test_transaction(
        &mut client,
        tenant_id,
        &cash_id,
        &revenue_id,
        "100.00",
        None,
        None,
    )
    .await;

    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: expense_id.clone(),
                amount: "30.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: cash_id.clone(),
                amount: "30.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };
    client.post_transaction(request).await.unwrap();

    // Get balances for all three accounts
    let request = GetBalancesRequest {
        tenant_id: tenant_id.to_string(),
        account_ids: vec![cash_id.clone(), revenue_id.clone(), expense_id.clone()],
        as_of_date: String::new(),
    };

    let response = client.get_balances(request).await.unwrap().into_inner();
    assert_eq!(response.balances.len(), 3);

    // Find each balance in the response
    let cash_balance = response
        .balances
        .iter()
        .find(|b| b.account_id == cash_id)
        .expect("Should have cash balance");
    let revenue_balance = response
        .balances
        .iter()
        .find(|b| b.account_id == revenue_id)
        .expect("Should have revenue balance");
    let expense_balance = response
        .balances
        .iter()
        .find(|b| b.account_id == expense_id)
        .expect("Should have expense balance");

    assert_eq!(cash_balance.balance, "70"); // 100 - 30
    assert_eq!(revenue_balance.balance, "100");
    assert_eq!(expense_balance.balance, "30");
}

/// Story 004, Task 3: Get balances skips non-existent accounts
#[tokio::test]
async fn get_balances_skips_non_existent() {
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
    let cash_id = cash.account.unwrap().account_id;

    // Request includes existing and non-existing accounts
    let request = GetBalancesRequest {
        tenant_id: tenant_id.to_string(),
        account_ids: vec![cash_id.clone(), Uuid::new_v4().to_string()],
        as_of_date: String::new(),
    };

    let response = client.get_balances(request).await.unwrap().into_inner();
    assert_eq!(
        response.balances.len(),
        1,
        "Should only return existing account"
    );
    assert_eq!(response.balances[0].account_id, cash_id);
}

/// Story 004: Verify balance reflects account type (debit-normal vs credit-normal)
#[tokio::test]
async fn balance_reflects_account_type() {
    let (mut client, tenant_id) = spawn_app().await;

    // Create all 5 account types
    let asset = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Asset,
        "ASSET",
        "USD",
        true,
    )
    .await;
    let liability = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Liability,
        "LIABILITY",
        "USD",
        true,
    )
    .await;
    let equity = create_test_account(
        &mut client,
        tenant_id,
        ProtoAccountType::Equity,
        "EQUITY",
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

    let asset_id = asset.account.unwrap().account_id;
    let liability_id = liability.account.unwrap().account_id;
    let equity_id = equity.account.unwrap().account_id;
    let revenue_id = revenue.account.unwrap().account_id;
    let expense_id = expense.account.unwrap().account_id;

    // Post transaction: Debit asset, credit equity (owner investment)
    post_test_transaction(
        &mut client,
        tenant_id,
        &asset_id,
        &equity_id,
        "1000.00",
        None,
        None,
    )
    .await;

    // Post transaction: Debit asset, credit revenue (sales)
    post_test_transaction(
        &mut client,
        tenant_id,
        &asset_id,
        &revenue_id,
        "500.00",
        None,
        None,
    )
    .await;

    // Post transaction: Debit expense, credit asset (payment)
    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: expense_id.clone(),
                amount: "200.00".to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: asset_id.clone(),
                amount: "200.00".to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: String::new(),
        idempotency_key: String::new(),
        metadata: String::new(),
    };
    client.post_transaction(request).await.unwrap();

    // Post transaction: Debit asset, credit liability (loan)
    post_test_transaction(
        &mut client,
        tenant_id,
        &asset_id,
        &liability_id,
        "300.00",
        None,
        None,
    )
    .await;

    // Verify balances
    // Asset: +1000 +500 -200 +300 = 1600 (debits increase, credits decrease)
    let bal = get_balance(&mut client, tenant_id, &asset_id, None).await;
    assert_eq!(bal.balance, "1600", "Asset balance");

    // Equity: +1000 (credit increases)
    let bal = get_balance(&mut client, tenant_id, &equity_id, None).await;
    assert_eq!(bal.balance, "1000", "Equity balance");

    // Revenue: +500 (credit increases)
    let bal = get_balance(&mut client, tenant_id, &revenue_id, None).await;
    assert_eq!(bal.balance, "500", "Revenue balance");

    // Expense: +200 (debit increases)
    let bal = get_balance(&mut client, tenant_id, &expense_id, None).await;
    assert_eq!(bal.balance, "200", "Expense balance");

    // Liability: +300 (credit increases)
    let bal = get_balance(&mut client, tenant_id, &liability_id, None).await;
    assert_eq!(bal.balance, "300", "Liability balance");

    // Verify accounting equation: Assets = Liabilities + Equity + (Revenue - Expense)
    // 1600 = 300 + 1000 + (500 - 200) = 300 + 1000 + 300 = 1600 ✓
}
