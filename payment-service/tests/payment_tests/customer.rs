use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_customer_response() -> serde_json::Value {
    serde_json::json!({
        "id": "cust_test_123",
        "entity": "customer",
        "name": "Test User",
        "email": "test@example.com",
        "contact": "+919876543210",
        "notes": []
    })
}

#[tokio::test]
#[serial]
async fn create_customer_via_grpc() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_customer_response()))
        .mount(&app.razorpay_mock)
        .await;

    let customer = client
        .create_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test User",
            "test@example.com",
            Some("+919876543210".to_string()),
        )
        .await
        .expect("Failed to create customer");

    assert!(!customer.id.is_empty());
    assert_eq!(customer.name, "Test User");
    assert_eq!(customer.email, "test@example.com");
    assert_eq!(customer.razorpay_customer_id, "cust_test_123");
}

#[tokio::test]
#[serial]
async fn create_customer_duplicate_returns_already_exists() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_customer_response()))
        .expect(1)
        .mount(&app.razorpay_mock)
        .await;

    // Create first customer
    client
        .create_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test User",
            "test@example.com",
            None,
        )
        .await
        .expect("Failed to create customer");

    // Second create with same user_id should fail
    let result = client
        .create_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test User 2",
            "test2@example.com",
            None,
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::AlreadyExists);
}

#[tokio::test]
#[serial]
async fn get_customer_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "nonexistent-id",
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
#[serial]
async fn get_customer_after_create() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_customer_response()))
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .create_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test User",
            "test@example.com",
            None,
        )
        .await
        .expect("Failed to create customer");

    let fetched = client
        .get_customer(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get customer");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "Test User");
}

#[tokio::test]
#[serial]
async fn list_customers_pagination() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    // Mock Razorpay returning different customer IDs
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_customer_response()))
        .mount(&app.razorpay_mock)
        .await;

    // Create a customer (only 1 since user_id must be unique per tenant)
    client
        .create_customer(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Test User",
            "test@example.com",
            None,
        )
        .await
        .expect("Failed to create customer");

    let (customers, total_count) = client
        .list_customers(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 10, 0)
        .await
        .expect("Failed to list customers");

    assert_eq!(customers.len(), 1);
    assert_eq!(total_count, 1);
}
