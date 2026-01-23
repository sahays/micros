//! Plan change integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create plans and subscription.
async fn create_test_setup(
    client: &mut billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    >,
) -> (Subscription, BillingPlan, BillingPlan) {
    // Create basic plan ($50/month)
    let basic_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Basic Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3, // Monthly
            interval_count: 1,
            base_price: "50.00".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );
    let basic_plan = client
        .create_plan(basic_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();

    // Create premium plan ($100/month)
    let premium_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Premium Plan".to_string(),
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
    let premium_plan = client
        .create_plan(premium_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();

    // Create subscription on basic plan
    let sub_request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "cccccccc-cccc-cccc-cccc-cccccccccccc".to_string(),
            plan_id: basic_plan.plan_id.clone(),
            billing_anchor_day: 1,
            start_date: "".to_string(),
            trial_end_date: "".to_string(),
            proration_mode: 1, // Immediate
            metadata: "".to_string(),
        },
    );
    let subscription = client
        .create_subscription(sub_request)
        .await
        .unwrap()
        .into_inner()
        .subscription
        .unwrap();

    (subscription, basic_plan, premium_plan)
}

#[tokio::test]
async fn change_plan_immediate_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, premium_plan) = create_test_setup(&mut client).await;

    // Change to premium plan with immediate proration
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            new_plan_id: premium_plan.plan_id.clone(),
            proration_mode: 1, // IMMEDIATE
        },
    );

    let response = client.change_plan(request).await;
    assert!(
        response.is_ok(),
        "ChangePlan should succeed: {:?}",
        response.err()
    );

    let resp = response.unwrap().into_inner();
    let updated = resp.subscription.unwrap();
    assert_eq!(updated.plan_id, premium_plan.plan_id);

    // Should have proration charges
    assert!(!resp.proration_charges.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn change_plan_next_cycle_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, premium_plan) = create_test_setup(&mut client).await;

    // Change to premium plan at next cycle
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            new_plan_id: premium_plan.plan_id.clone(),
            proration_mode: 2, // NEXT_CYCLE
        },
    );

    let response = client.change_plan(request).await;
    assert!(response.is_ok());

    let resp = response.unwrap().into_inner();
    let updated = resp.subscription.unwrap();

    // Pending plan should be set
    assert_eq!(updated.pending_plan_id, premium_plan.plan_id);

    // No proration charges for next_cycle mode
    assert!(resp.proration_charges.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn change_plan_none_mode_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, premium_plan) = create_test_setup(&mut client).await;

    // Change to premium plan without proration
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            new_plan_id: premium_plan.plan_id.clone(),
            proration_mode: 3, // NONE
        },
    );

    let response = client.change_plan(request).await;
    assert!(response.is_ok());

    let resp = response.unwrap().into_inner();
    let updated = resp.subscription.unwrap();

    // Plan changed immediately
    assert_eq!(updated.plan_id, premium_plan.plan_id);

    // No proration charges for none mode
    assert!(resp.proration_charges.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn change_to_archived_plan_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, premium_plan) = create_test_setup(&mut client).await;

    // Archive the premium plan
    let archive_request = with_tenant(
        TEST_TENANT_ID,
        ArchivePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            plan_id: premium_plan.plan_id.clone(),
        },
    );
    client.archive_plan(archive_request).await.unwrap();

    // Try to change to archived plan
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            new_plan_id: premium_plan.plan_id,
            proration_mode: 1,
        },
    );

    let response = client.change_plan(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );

    app.cleanup().await;
}

#[tokio::test]
async fn change_to_different_currency_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, _premium_plan) = create_test_setup(&mut client).await;

    // Create plan with different currency
    let eur_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "EUR Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "75.00".to_string(),
            currency: "EUR".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );
    let eur_plan = client
        .create_plan(eur_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();

    // Try to change to EUR plan (subscription is USD)
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            new_plan_id: eur_plan.plan_id,
            proration_mode: 1,
        },
    );

    let response = client.change_plan(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);

    app.cleanup().await;
}

#[tokio::test]
async fn change_cancelled_subscription_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _basic_plan, premium_plan) = create_test_setup(&mut client).await;

    // Cancel subscription
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

    // Try to change plan
    let request = with_tenant(
        TEST_TENANT_ID,
        ChangePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            new_plan_id: premium_plan.plan_id,
            proration_mode: 1,
        },
    );

    let response = client.change_plan(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );

    app.cleanup().await;
}
