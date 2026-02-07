mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_account_response() -> serde_json::Value {
    serde_json::json!({
        "id": "acc_test_123",
        "type": "route",
        "status": "created",
        "email": "vendor@example.com",
        "legal_business_name": "Vendor Inc",
        "contact_name": "Test Vendor"
    })
}

fn mock_razorpay_direct_transfer_response() -> serde_json::Value {
    serde_json::json!({
        "id": "trf_direct_123",
        "entity": "transfer",
        "amount": 3000,
        "currency": "INR",
        "status": "created",
        "recipient": "acc_test_123",
        "amount_reversed": 0,
        "on_hold": false
    })
}

#[tokio::test]
async fn create_transfer_from_payment_requires_completed_payment() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a linked account first
    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    let account = client
        .create_linked_account(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            service_core::grpc::proto::payment::BankAccount {
                account_holder_name: "Test Vendor".to_string(),
                account_number: "1234567890".to_string(),
                ifsc_code: "SBIN0001234".to_string(),
                account_type: "savings".to_string(),
            },
            service_core::grpc::proto::payment::LegalInfo {
                legal_business_name: "Vendor Inc".to_string(),
                business_type: "individual".to_string(),
                pan: Some("ABCDE1234F".to_string()),
                gst: None,
            },
            None,
        )
        .await
        .expect("Failed to create linked account");

    // Create a transaction (status = Created, not Completed)
    let tx = client
        .create_transaction(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 10000, "INR")
        .await
        .expect("Failed to create transaction");

    // Try to create transfer from non-completed payment
    let result = client
        .create_transfer_from_payment(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            &tx.id,
            &account.id,
            5000,
            "INR",
            false,
            None,
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn create_direct_transfer_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a linked account
    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    // Activate the account manually by updating status in DB
    let account = client
        .create_linked_account(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            service_core::grpc::proto::payment::BankAccount {
                account_holder_name: "Test Vendor".to_string(),
                account_number: "1234567890".to_string(),
                ifsc_code: "SBIN0001234".to_string(),
                account_type: "savings".to_string(),
            },
            service_core::grpc::proto::payment::LegalInfo {
                legal_business_name: "Vendor Inc".to_string(),
                business_type: "individual".to_string(),
                pan: Some("ABCDE1234F".to_string()),
                gst: None,
            },
            None,
        )
        .await
        .expect("Failed to create linked account");

    // Activate the account directly in DB
    app.db
        .collection::<mongodb::bson::Document>("linked_accounts")
        .update_one(
            mongodb::bson::doc! { "_id": &account.id },
            mongodb::bson::doc! { "$set": { "status": "ACTIVATED" } },
            None,
        )
        .await
        .expect("Failed to activate account");

    // Mock the transfer API
    Mock::given(method("POST"))
        .and(path_regex("/v1/transfers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_direct_transfer_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    let transfer = client
        .create_direct_transfer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            &account.id,
            3000,
            "INR",
        )
        .await
        .expect("Failed to create direct transfer");

    assert!(!transfer.id.is_empty());
    assert_eq!(transfer.amount, 3000);
    assert_eq!(transfer.currency, "INR");
    assert_eq!(transfer.linked_account_id, account.id);

    app.cleanup().await;
}

#[tokio::test]
async fn get_transfer_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_transfer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "nonexistent-id",
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_transfers_empty() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (transfers, total) = client
        .list_transfers(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            None,
            None,
            None,
            10,
            0,
        )
        .await
        .expect("Failed to list transfers");

    assert_eq!(transfers.len(), 0);
    assert_eq!(total, 0);

    app.cleanup().await;
}
