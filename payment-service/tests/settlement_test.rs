mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_settlement_response() -> serde_json::Value {
    serde_json::json!({
        "id": "setl_test_123",
        "entity": "settlement",
        "amount": 100000,
        "status": "created",
        "currency": "INR",
        "utr": null,
        "created_at": 1700000000
    })
}

#[tokio::test]
async fn request_on_demand_settlement_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/settlements/ondemand"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_settlement_response()))
        .mount(&app.razorpay_mock)
        .await;

    let settlement = client
        .request_on_demand_settlement(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            None,
            100000,
            "INR",
        )
        .await
        .expect("Failed to request on-demand settlement");

    assert!(!settlement.id.is_empty());
    assert_eq!(settlement.amount, 100000);
    assert_eq!(settlement.currency, "INR");
    assert_eq!(settlement.razorpay_settlement_id, "setl_test_123");

    app.cleanup().await;
}

#[tokio::test]
async fn get_settlement_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_settlement(
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
async fn get_settlement_after_create() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/settlements/ondemand"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_settlement_response()))
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .request_on_demand_settlement(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            None,
            100000,
            "INR",
        )
        .await
        .expect("Failed to request settlement");

    let fetched = client
        .get_settlement(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get settlement");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.amount, 100000);

    app.cleanup().await;
}

#[tokio::test]
async fn list_settlements_empty() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (settlements, total) = client
        .list_settlements(
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
        .expect("Failed to list settlements");

    assert_eq!(settlements.len(), 0);
    assert_eq!(total, 0);

    app.cleanup().await;
}
