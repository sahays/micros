use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_TENANT_ID, TEST_USER_ID};
use serial_test::serial;
use service_core::grpc::proto::payment::{RefundSpeed, TransactionStatus};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_refund_response() -> serde_json::Value {
    serde_json::json!({
        "id": "rfnd_test_123",
        "entity": "refund",
        "amount": 10000,
        "currency": "INR",
        "payment_id": "pay_test_123",
        "status": "created",
        "speed_requested": "normal",
        "created_at": 1700000000
    })
}

/// Helper to set up a completed payment in the test DB.
async fn create_completed_payment(
    app: &TestApp,
    client: &mut service_core::grpc::PaymentClient,
) -> String {
    // Create transaction
    let tx = client
        .create_transaction(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), 10000, "INR")
        .await
        .expect("Failed to create transaction");

    // Mark it as Completed with a provider_order_id
    app.db
        .collection::<mongodb::bson::Document>("transactions")
        .update_one(
            mongodb::bson::doc! { "_id": &tx.id },
            mongodb::bson::doc! {
                "$set": {
                    "status": "COMPLETED",
                    "provider_order_id": "pay_test_123"
                }
            },
            None,
        )
        .await
        .expect("Failed to update transaction status");

    tx.id
}

#[tokio::test]
#[serial]
async fn initiate_full_refund_updates_transaction_to_refunded() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let payment_id = create_completed_payment(app, &mut client).await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payments/.+/refunds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_refund_response()))
        .mount(&app.razorpay_mock)
        .await;

    // Full refund (amount = 0 means full refund)
    let refund = client
        .initiate_refund(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            &payment_id,
            0,
            RefundSpeed::Normal,
            Some("Customer request".to_string()),
            false,
        )
        .await
        .expect("Failed to initiate refund");

    assert!(!refund.id.is_empty());
    assert_eq!(refund.razorpay_refund_id, "rfnd_test_123");
    assert_eq!(refund.payment_id, payment_id);

    // Verify the transaction was updated to Refunded
    let tx = client
        .get_transaction(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &payment_id)
        .await
        .expect("Failed to get transaction");

    assert_eq!(tx.status, TransactionStatus::Refunded as i32);
}

#[tokio::test]
#[serial]
async fn initiate_partial_refund_updates_transaction_to_partially_refunded() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let payment_id = create_completed_payment(app, &mut client).await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payments/.+/refunds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "rfnd_partial_123",
            "entity": "refund",
            "amount": 5000,
            "currency": "INR",
            "payment_id": "pay_test_123",
            "status": "created",
            "speed_requested": "normal",
            "created_at": 1700000000
        })))
        .mount(&app.razorpay_mock)
        .await;

    // Partial refund (50 INR of 100 INR = 5000 paise of 10000 paise)
    let refund = client
        .initiate_refund(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            &payment_id,
            5000,
            RefundSpeed::Normal,
            None,
            false,
        )
        .await
        .expect("Failed to initiate refund");

    assert_eq!(refund.amount, 5000);

    // Verify the transaction was updated to PartiallyRefunded
    let tx = client
        .get_transaction(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &payment_id)
        .await
        .expect("Failed to get transaction");

    assert_eq!(tx.status, TransactionStatus::PartiallyRefunded as i32);
}

#[tokio::test]
#[serial]
async fn refund_exceeds_available_amount_returns_invalid_argument() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let payment_id = create_completed_payment(app, &mut client).await;

    // Try to refund more than the payment amount (10000 paise = 100 INR)
    let result = client
        .initiate_refund(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            &payment_id,
            20000, // More than the 10000 paise payment
            RefundSpeed::Normal,
            None,
            false,
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
#[serial]
async fn refund_non_completed_payment_returns_failed_precondition() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    // Create a transaction (status = Created, not Completed)
    let tx = client
        .create_transaction(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), 10000, "INR")
        .await
        .expect("Failed to create transaction");

    let result = client
        .initiate_refund(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            &tx.id,
            0,
            RefundSpeed::Normal,
            None,
            false,
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);
}

#[tokio::test]
#[serial]
async fn get_refund_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_refund(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "nonexistent-id",
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
#[serial]
async fn list_refunds_empty() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let (refunds, total) = client
        .list_refunds(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            None,
            None,
            10,
            0,
        )
        .await
        .expect("Failed to list refunds");

    assert_eq!(refunds.len(), 0);
    assert_eq!(total, 0);
}
