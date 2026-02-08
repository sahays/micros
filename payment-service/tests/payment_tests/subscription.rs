use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use serial_test::serial;
use service_core::grpc::proto::payment::PlanPeriod;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_plan_response() -> serde_json::Value {
    serde_json::json!({
        "id": "plan_test_123",
        "entity": "plan",
        "interval": 1,
        "period": "monthly",
        "item": {
            "id": "item_test_123",
            "active": true,
            "amount": 50000,
            "unit_amount": 50000,
            "currency": "INR",
            "name": "Basic Plan",
            "description": "Basic monthly plan"
        },
        "notes": [],
        "created_at": 1700000000
    })
}

fn mock_razorpay_subscription_response() -> serde_json::Value {
    serde_json::json!({
        "id": "sub_test_123",
        "entity": "subscription",
        "plan_id": "plan_test_123",
        "status": "created",
        "total_count": 12,
        "paid_count": 0,
        "remaining_count": 12,
        "short_url": "https://rzp.io/i/test123",
        "charge_at": 1700000000,
        "created_at": 1700000000
    })
}

#[tokio::test]
#[serial]
async fn create_plan_via_grpc() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/plans"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_plan_response()))
        .mount(&app.razorpay_mock)
        .await;

    let plan = client
        .create_razorpay_plan(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Basic Plan",
            "Basic monthly plan",
            50000,
            "INR",
            PlanPeriod::Monthly,
            1,
        )
        .await
        .expect("Failed to create plan");

    assert!(!plan.id.is_empty());
    assert_eq!(plan.name, "Basic Plan");
    assert_eq!(plan.amount, 50000);
    assert_eq!(plan.currency, "INR");
    assert_eq!(plan.razorpay_plan_id, "plan_test_123");
}

#[tokio::test]
#[serial]
async fn get_plan_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_razorpay_plan(
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
async fn create_subscription_returns_short_url() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    // Mock plan creation
    Mock::given(method("POST"))
        .and(path_regex("/v1/plans"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_plan_response()))
        .mount(&app.razorpay_mock)
        .await;

    // Mock subscription creation
    Mock::given(method("POST"))
        .and(path_regex("/v1/subscriptions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_razorpay_subscription_response()),
        )
        .mount(&app.razorpay_mock)
        .await;

    // Create a plan first
    let plan = client
        .create_razorpay_plan(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Basic Plan",
            "Basic monthly plan",
            50000,
            "INR",
            PlanPeriod::Monthly,
            1,
        )
        .await
        .expect("Failed to create plan");

    // Create a subscription
    let subscription = client
        .create_razorpay_subscription(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            &plan.id,
            None,
            12,
            None,
        )
        .await
        .expect("Failed to create subscription");

    assert!(!subscription.id.is_empty());
    assert_eq!(subscription.razorpay_subscription_id, "sub_test_123");
    assert_eq!(
        subscription.short_url,
        Some("https://rzp.io/i/test123".to_string())
    );
    assert_eq!(subscription.total_count, 12);
}

#[tokio::test]
#[serial]
async fn list_plans_pagination() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v1/plans"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_plan_response()))
        .mount(&app.razorpay_mock)
        .await;

    client
        .create_razorpay_plan(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "Basic Plan",
            "Basic monthly plan",
            50000,
            "INR",
            PlanPeriod::Monthly,
            1,
        )
        .await
        .expect("Failed to create plan");

    let (plans, total) = client
        .list_razorpay_plans(TEST_APP_ID, TEST_ORG_ID, Some(TEST_USER_ID), 10, 0)
        .await
        .expect("Failed to list plans");

    assert_eq!(plans.len(), 1);
    assert_eq!(total, 1);
}

#[tokio::test]
#[serial]
async fn get_subscription_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_razorpay_subscription(
            TEST_APP_ID,
            TEST_ORG_ID,
            Some(TEST_USER_ID),
            "nonexistent-id",
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
}
