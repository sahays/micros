//! Payment + Ledger Workflow Tests
//!
//! Tests payment processing and ledger integration.
//! Payment gateway (Razorpay) is mocked in tests.

mod common;

use tonic::Request;
use uuid::Uuid;
use workflow_tests::proto::payment::{
    CreateTransactionRequest, UpdateTransactionStatusRequest,
    TransactionStatus, GetTransactionRequest, ListTransactionsRequest,
};
use workflow_tests::ServiceEndpoints;

/// Helper to create a payment transaction.
async fn create_test_transaction(
    tenant_id: &str,
    user_id: &str,
    amount: f64,
) -> String {
    let endpoints = ServiceEndpoints::from_env();
    let mut payment_client = workflow_tests::PaymentServiceClient::connect(endpoints.payment.clone())
        .await
        .expect("Failed to connect to payment service");

    let mut request = Request::new(CreateTransactionRequest {
        amount,
        currency: "INR".to_string(),
    });

    request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let response = payment_client
        .create_transaction(request)
        .await
        .expect("Failed to create transaction");

    response.into_inner().transaction.unwrap().id
}

/// Test: Payment transaction can be created.
#[tokio::test]
async fn create_payment_transaction() {
    common::setup().await;

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let transaction_id = create_test_transaction(&tenant_id, &user_id, 1000.00).await;
    assert!(!transaction_id.is_empty());

    // Verify transaction can be retrieved
    let endpoints = ServiceEndpoints::from_env();
    let mut payment_client = workflow_tests::PaymentServiceClient::connect(endpoints.payment.clone())
        .await
        .expect("Failed to connect to payment service");

    let mut get_request = Request::new(GetTransactionRequest {
        transaction_id: transaction_id.clone(),
    });

    get_request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    get_request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    get_request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    get_request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let response = payment_client
        .get_transaction(get_request)
        .await
        .expect("Failed to get transaction");

    let transaction = response.into_inner().transaction.unwrap();
    assert_eq!(transaction.id, transaction_id);
    assert_eq!(transaction.amount, 1000.00);
}

/// Test: Payment status can be updated.
#[tokio::test]
async fn update_payment_status() {
    common::setup().await;

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let transaction_id = create_test_transaction(&tenant_id, &user_id, 500.00).await;

    let endpoints = ServiceEndpoints::from_env();
    let mut payment_client = workflow_tests::PaymentServiceClient::connect(endpoints.payment.clone())
        .await
        .expect("Failed to connect to payment service");

    // Update status to completed
    let mut update_request = Request::new(UpdateTransactionStatusRequest {
        transaction_id: transaction_id.clone(),
        status: TransactionStatus::Completed as i32,
    });

    update_request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    update_request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    update_request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    update_request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let _response = payment_client
        .update_transaction_status(update_request)
        .await
        .expect("Failed to update transaction status");

    // Verify status was updated
    let mut get_request = Request::new(GetTransactionRequest {
        transaction_id: transaction_id.clone(),
    });

    get_request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    get_request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    get_request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    get_request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let response = payment_client
        .get_transaction(get_request)
        .await
        .expect("Failed to get transaction");

    let transaction = response.into_inner().transaction.unwrap();
    assert_eq!(transaction.status, TransactionStatus::Completed as i32);
}

/// Test: Payment transactions can be listed with filters.
#[tokio::test]
async fn list_payment_transactions() {
    common::setup().await;

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    // Create multiple transactions
    for amount in [100.00, 200.00, 300.00] {
        create_test_transaction(&tenant_id, &user_id, amount).await;
    }

    let endpoints = ServiceEndpoints::from_env();
    let mut payment_client = workflow_tests::PaymentServiceClient::connect(endpoints.payment.clone())
        .await
        .expect("Failed to connect to payment service");

    let mut list_request = Request::new(ListTransactionsRequest {
        status: None,
        limit: 100,
        offset: 0,
    });

    list_request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    list_request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    list_request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    list_request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let response = payment_client
        .list_transactions(list_request)
        .await
        .expect("Failed to list transactions");

    let transactions = response.into_inner().transactions;
    assert!(transactions.len() >= 3);
}

/// Test: Tenant isolation in payment transactions.
#[tokio::test]
async fn payment_tenant_isolation() {
    common::setup().await;

    let tenant_a_id = Uuid::new_v4().to_string();
    let user_a_id = Uuid::new_v4().to_string();
    let tenant_b_id = Uuid::new_v4().to_string();
    let user_b_id = Uuid::new_v4().to_string();

    // Create transaction in Tenant A
    let transaction_id = create_test_transaction(&tenant_a_id, &user_a_id, 999.00).await;

    // Try to access from Tenant B
    let endpoints = ServiceEndpoints::from_env();
    let mut payment_client = workflow_tests::PaymentServiceClient::connect(endpoints.payment.clone())
        .await
        .expect("Failed to connect to payment service");

    let mut get_request = Request::new(GetTransactionRequest {
        transaction_id: transaction_id.clone(),
    });

    get_request.metadata_mut().insert("x-tenant-id", tenant_b_id.parse().unwrap());
    get_request.metadata_mut().insert("x-user-id", user_b_id.parse().unwrap());
    get_request.metadata_mut().insert("x-app-id", tenant_b_id.parse().unwrap());
    get_request.metadata_mut().insert("x-org-id", tenant_b_id.parse().unwrap());

    let response = payment_client.get_transaction(get_request).await;

    // Should not find the transaction
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}
