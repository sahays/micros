use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_TENANT_ID, TEST_USER_ID};
use serial_test::serial;
use service_core::grpc::proto::payment::{PaymentChannel, PaymentMethodType, TransactionStatus};

// =========================================================================
// Direct UPI Payment Tests
// =========================================================================

#[tokio::test]
#[serial]
async fn record_direct_upi_payment_creates_completed_transaction() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let tx = client
        .record_direct_upi_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            50000, // 500.00 INR
            "INR",
            "UTR123456789012",
            Some("payer@upi".to_string()),
            Some("Direct UPI payment".to_string()),
        )
        .await
        .expect("Failed to record direct UPI payment");

    assert_eq!(tx.status(), TransactionStatus::Completed);
    assert_eq!(tx.amount_paise, 50000);
    assert_eq!(tx.currency, "INR");
    assert_eq!(tx.app_id, TEST_APP_ID);
    assert_eq!(tx.tenant_id, TEST_TENANT_ID);
    assert_eq!(tx.user_id, Some(TEST_USER_ID.to_string()));
    assert_eq!(tx.payment_channel(), PaymentChannel::DirectUpi);
    assert_eq!(tx.payment_method_type(), PaymentMethodType::Upi);
    assert_eq!(tx.external_reference, Some("UTR123456789012".to_string()));
    assert_eq!(tx.notes, Some("Direct UPI payment".to_string()));
    assert!(!tx.id.is_empty());

    // Verify we can fetch it back
    let fetched = client
        .get_transaction(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &tx.id)
        .await
        .expect("Failed to get transaction");
    assert_eq!(fetched.id, tx.id);
    assert_eq!(fetched.payment_channel(), PaymentChannel::DirectUpi);
}

#[tokio::test]
#[serial]
async fn record_direct_upi_payment_rejects_duplicate_utr() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    // First recording should succeed
    client
        .record_direct_upi_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "UTR_DUPLICATE_TEST",
            None,
            None,
        )
        .await
        .expect("First recording should succeed");

    // Second recording with same UTR should fail
    let err = client
        .record_direct_upi_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            20000,
            "INR",
            "UTR_DUPLICATE_TEST",
            None,
            None,
        )
        .await
        .expect_err("Duplicate UTR should be rejected");

    assert_eq!(err.code(), tonic::Code::AlreadyExists);
    assert!(err.message().contains("UTR_DUPLICATE_TEST"));
}

#[tokio::test]
#[serial]
async fn record_direct_upi_payment_rejects_zero_amount() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let err = client
        .record_direct_upi_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            0,
            "INR",
            "UTR_ZERO_AMOUNT",
            None,
            None,
        )
        .await
        .expect_err("Zero amount should be rejected");

    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("Amount"));
}

#[tokio::test]
#[serial]
async fn record_direct_upi_payment_rejects_empty_utr() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let err = client
        .record_direct_upi_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "",
            None,
            None,
        )
        .await
        .expect_err("Empty UTR should be rejected");

    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("UTR"));
}

// =========================================================================
// Offline Payment Tests
// =========================================================================

#[tokio::test]
#[serial]
async fn record_offline_cash_payment_creates_completed_transaction() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let tx = client
        .record_offline_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            25000, // 250.00 INR
            "INR",
            PaymentMethodType::Cash,
            None,
            Some("Cash payment received at counter".to_string()),
        )
        .await
        .expect("Failed to record offline cash payment");

    assert_eq!(tx.status(), TransactionStatus::Completed);
    assert_eq!(tx.amount_paise, 25000);
    assert_eq!(tx.currency, "INR");
    assert_eq!(tx.payment_channel(), PaymentChannel::Offline);
    assert_eq!(tx.payment_method_type(), PaymentMethodType::Cash);
    assert_eq!(tx.external_reference, None);
    assert_eq!(
        tx.notes,
        Some("Cash payment received at counter".to_string())
    );
}

#[tokio::test]
#[serial]
async fn record_offline_cheque_payment_with_reference() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let tx = client
        .record_offline_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            100000, // 1000.00 INR
            "INR",
            PaymentMethodType::Cheque,
            Some("CHQ-2024-001234".to_string()),
            Some("Cheque from ABC Corp".to_string()),
        )
        .await
        .expect("Failed to record offline cheque payment");

    assert_eq!(tx.status(), TransactionStatus::Completed);
    assert_eq!(tx.payment_channel(), PaymentChannel::Offline);
    assert_eq!(tx.payment_method_type(), PaymentMethodType::Cheque);
    assert_eq!(tx.external_reference, Some("CHQ-2024-001234".to_string()));
}

#[tokio::test]
#[serial]
async fn record_offline_payment_rejects_duplicate_external_ref() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    // First recording should succeed
    client
        .record_offline_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            50000,
            "INR",
            PaymentMethodType::BankTransfer,
            Some("NEFT-REF-001".to_string()),
            None,
        )
        .await
        .expect("First recording should succeed");

    // Second recording with same external_reference should fail
    let err = client
        .record_offline_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            30000,
            "INR",
            PaymentMethodType::BankTransfer,
            Some("NEFT-REF-001".to_string()),
            None,
        )
        .await
        .expect_err("Duplicate external reference should be rejected");

    assert_eq!(err.code(), tonic::Code::AlreadyExists);
    assert!(err.message().contains("NEFT-REF-001"));
}

#[tokio::test]
#[serial]
async fn record_offline_payment_rejects_zero_amount() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let err = client
        .record_offline_payment(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            0,
            "INR",
            PaymentMethodType::Cash,
            None,
            None,
        )
        .await
        .expect_err("Zero amount should be rejected");

    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("Amount"));
}
