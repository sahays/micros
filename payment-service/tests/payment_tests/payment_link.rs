use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_TENANT_ID, TEST_USER_ID};
use serial_test::serial;
use service_core::grpc::proto::payment::PaymentLinkStatus;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_payment_link_response() -> serde_json::Value {
    serde_json::json!({
        "id": "plink_test_123",
        "amount": 10000,
        "currency": "INR",
        "description": "Test Payment",
        "status": "created",
        "short_url": "https://rzp.io/i/paylink123",
        "amount_paid": 0,
        "created_at": 1700000000
    })
}

fn mock_razorpay_cancel_response() -> serde_json::Value {
    serde_json::json!({
        "id": "plink_test_123",
        "status": "cancelled",
        "amount": 10000,
        "currency": "INR",
        "short_url": "https://rzp.io/i/paylink123"
    })
}

#[tokio::test]
#[serial]
async fn create_payment_link_via_grpc() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payment_links"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_payment_link_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    let link = client
        .create_payment_link(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "Test Payment",
            Some("John Doe".to_string()),
            Some("john@example.com".to_string()),
            None,
            None,
        )
        .await
        .expect("Failed to create payment link");

    assert!(!link.id.is_empty());
    assert_eq!(link.amount, 10000);
    assert_eq!(link.currency, "INR");
    assert_eq!(link.description, "Test Payment");
    assert_eq!(link.short_url, "https://rzp.io/i/paylink123");
    assert_eq!(link.status, PaymentLinkStatus::Created as i32);
}

#[tokio::test]
#[serial]
async fn get_payment_link_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_payment_link(
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
async fn get_payment_link_after_create() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payment_links"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_payment_link_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .create_payment_link(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "Test Payment",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create payment link");

    let fetched = client
        .get_payment_link(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get payment link");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.amount, 10000);
}

#[tokio::test]
#[serial]
async fn cancel_payment_link_created_status() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payment_links$"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_payment_link_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payment_links/.+/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_cancel_response()))
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .create_payment_link(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "Test Payment",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create payment link");

    let cancelled = client
        .cancel_payment_link(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to cancel payment link");

    assert_eq!(cancelled.status, PaymentLinkStatus::Cancelled as i32);
}

#[tokio::test]
#[serial]
async fn list_payment_links_by_status() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/payment_links"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_payment_link_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    client
        .create_payment_link(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            10000,
            "INR",
            "Test Payment",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to create payment link");

    // List all
    let (links, total) = client
        .list_payment_links(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), None, 10, 0)
        .await
        .expect("Failed to list payment links");

    assert_eq!(links.len(), 1);
    assert_eq!(total, 1);

    // List by PAID status (none should match)
    let (links, total) = client
        .list_payment_links(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            Some(PaymentLinkStatus::Paid),
            10,
            0,
        )
        .await
        .expect("Failed to list payment links");

    assert_eq!(links.len(), 0);
    assert_eq!(total, 0);
}
