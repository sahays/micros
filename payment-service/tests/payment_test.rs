mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use service_core::grpc::proto::payment::TransactionStatus;

#[tokio::test]
async fn create_transaction_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let transaction = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 100.0, "INR")
        .await
        .expect("Failed to create transaction");

    assert!(!transaction.id.is_empty());
    assert_eq!(transaction.amount, 100.0);
    assert_eq!(transaction.currency, "INR");
    assert_eq!(transaction.status, TransactionStatus::Created as i32);

    app.cleanup().await;
}

#[tokio::test]
async fn get_transaction_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a transaction first
    let created = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 250.50, "INR")
        .await
        .expect("Failed to create transaction");

    // Get the transaction
    let fetched = client
        .get_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get transaction");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.amount, 250.50);
    assert_eq!(fetched.currency, "INR");

    app.cleanup().await;
}

#[tokio::test]
async fn update_transaction_status_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a transaction
    let created = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 500.0, "INR")
        .await
        .expect("Failed to create transaction");

    // Update status to Pending
    client
        .update_transaction_status(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            &created.id,
            TransactionStatus::Pending,
        )
        .await
        .expect("Failed to update transaction status");

    // Verify the status was updated
    let fetched = client
        .get_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get transaction");

    assert_eq!(fetched.status, TransactionStatus::Pending as i32);

    app.cleanup().await;
}

#[tokio::test]
async fn list_transactions_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create multiple transactions
    for i in 0..5 {
        client
            .create_transaction(
                TEST_APP_ID,
                TEST_ORG_ID,
                Some(TEST_USER_ID),
                (i + 1) as f64 * 100.0,
                "INR",
            )
            .await
            .expect("Failed to create transaction");
    }

    // List all transactions
    let (transactions, total_count) = client
        .list_transactions(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), None, 10, 0)
        .await
        .expect("Failed to list transactions");

    assert_eq!(transactions.len(), 5);
    assert_eq!(total_count, 5);

    app.cleanup().await;
}

#[tokio::test]
async fn list_transactions_with_status_filter() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create transactions with different statuses
    let tx1 = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 100.0, "INR")
        .await
        .expect("Failed to create transaction");

    let tx2 = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 200.0, "INR")
        .await
        .expect("Failed to create transaction");

    // Update one to Completed
    client
        .update_transaction_status(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            &tx1.id,
            TransactionStatus::Completed,
        )
        .await
        .expect("Failed to update status");

    // List only Completed transactions
    let (completed, _) = client
        .list_transactions(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            Some(TransactionStatus::Completed),
            10,
            0,
        )
        .await
        .expect("Failed to list transactions");

    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, tx1.id);

    // List only Created transactions
    let (created, _) = client
        .list_transactions(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            Some(TransactionStatus::Created),
            10,
            0,
        )
        .await
        .expect("Failed to list transactions");

    assert_eq!(created.len(), 1);
    assert_eq!(created[0].id, tx2.id);

    app.cleanup().await;
}

#[tokio::test]
async fn get_nonexistent_transaction_returns_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_transaction(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "00000000-0000-0000-0000-000000000000",
        )
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_upi_qr_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let response = client
        .generate_upi_qr(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            100.0,
            Some("Test payment".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("Failed to generate UPI QR");

    assert!(response.upi_link.starts_with("upi://pay"));
    assert!(response.upi_link.contains("am=100.00"));

    app.cleanup().await;
}
