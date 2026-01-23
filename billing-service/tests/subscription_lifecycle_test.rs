//! Subscription lifecycle integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create a plan and subscription for lifecycle tests.
async fn create_test_subscription(
    client: &mut billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    >,
    trial: bool,
) -> Subscription {
    // Create plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Lifecycle Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "100.00".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );
    let plan = client
        .create_plan(plan_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();

    // Create subscription
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "66666666-6666-6666-6666-666666666666".to_string(),
            plan_id: plan.plan_id,
            billing_anchor_day: 1,
            start_date: "".to_string(),
            trial_end_date: if trial {
                "2026-02-28".to_string()
            } else {
                "".to_string()
            },
            proration_mode: 1,
            metadata: "".to_string(),
        },
    );

    client
        .create_subscription(request)
        .await
        .unwrap()
        .into_inner()
        .subscription
        .unwrap()
}

#[tokio::test]
async fn activate_trial_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, true).await;
    assert_eq!(subscription.status, 1); // TRIAL

    // Activate
    let request = with_tenant(
        TEST_TENANT_ID,
        ActivateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
        },
    );

    let response = client.activate_subscription(request).await;
    assert!(
        response.is_ok(),
        "ActivateSubscription should succeed: {:?}",
        response.err()
    );

    let activated = response.unwrap().into_inner().subscription.unwrap();
    assert_eq!(activated.status, 2); // ACTIVE

    app.cleanup().await;
}

#[tokio::test]
async fn activate_active_subscription_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;
    assert_eq!(subscription.status, 2); // ACTIVE

    // Try to activate already active subscription
    let request = with_tenant(
        TEST_TENANT_ID,
        ActivateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
        },
    );

    let response = client.activate_subscription(request).await;
    assert!(response.is_err());
    let err = response.unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn pause_active_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;
    assert_eq!(subscription.status, 2); // ACTIVE

    // Pause
    let request = with_tenant(
        TEST_TENANT_ID,
        PauseSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            reason: "Customer requested pause".to_string(),
        },
    );

    let response = client.pause_subscription(request).await;
    assert!(response.is_ok());

    let paused = response.unwrap().into_inner().subscription.unwrap();
    assert_eq!(paused.status, 3); // PAUSED

    app.cleanup().await;
}

#[tokio::test]
async fn pause_paused_subscription_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;

    // Pause first
    let pause_request = with_tenant(
        TEST_TENANT_ID,
        PauseSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            reason: "".to_string(),
        },
    );
    client.pause_subscription(pause_request).await.unwrap();

    // Try to pause again
    let request = with_tenant(
        TEST_TENANT_ID,
        PauseSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            reason: "".to_string(),
        },
    );

    let response = client.pause_subscription(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );

    app.cleanup().await;
}

#[tokio::test]
async fn resume_paused_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;

    // Pause first
    let pause_request = with_tenant(
        TEST_TENANT_ID,
        PauseSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            reason: "".to_string(),
        },
    );
    client.pause_subscription(pause_request).await.unwrap();

    // Resume
    let request = with_tenant(
        TEST_TENANT_ID,
        ResumeSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
        },
    );

    let response = client.resume_subscription(request).await;
    assert!(response.is_ok());

    let resumed = response.unwrap().into_inner().subscription.unwrap();
    assert_eq!(resumed.status, 2); // ACTIVE

    app.cleanup().await;
}

#[tokio::test]
async fn resume_active_subscription_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;
    assert_eq!(subscription.status, 2); // ACTIVE

    // Try to resume active subscription
    let request = with_tenant(
        TEST_TENANT_ID,
        ResumeSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
        },
    );

    let response = client.resume_subscription(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );

    app.cleanup().await;
}

#[tokio::test]
async fn cancel_subscription_immediately_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;

    // Cancel immediately
    let request = with_tenant(
        TEST_TENANT_ID,
        CancelSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            cancel_at_period_end: false,
            reason: "Customer churn".to_string(),
        },
    );

    let response = client.cancel_subscription(request).await;
    assert!(response.is_ok());

    let cancelled = response.unwrap().into_inner().subscription.unwrap();
    assert_eq!(cancelled.status, 4); // CANCELLED

    app.cleanup().await;
}

#[tokio::test]
async fn cancel_subscription_at_period_end_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;

    // Cancel at period end
    let request = with_tenant(
        TEST_TENANT_ID,
        CancelSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            cancel_at_period_end: true,
            reason: "".to_string(),
        },
    );

    let response = client.cancel_subscription(request).await;
    assert!(response.is_ok());

    let cancelled = response.unwrap().into_inner().subscription.unwrap();
    // Status should remain active until period end
    assert_eq!(cancelled.status, 2); // ACTIVE
                                     // But end_date should be set
    assert!(!cancelled.end_date.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn cancel_cancelled_subscription_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, false).await;

    // Cancel first
    let cancel_request = with_tenant(
        TEST_TENANT_ID,
        CancelSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            cancel_at_period_end: false,
            reason: "".to_string(),
        },
    );
    client.cancel_subscription(cancel_request).await.unwrap();

    // Try to cancel again
    let request = with_tenant(
        TEST_TENANT_ID,
        CancelSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            cancel_at_period_end: false,
            reason: "".to_string(),
        },
    );

    let response = client.cancel_subscription(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );

    app.cleanup().await;
}
