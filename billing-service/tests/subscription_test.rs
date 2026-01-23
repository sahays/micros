//! Subscription CRUD integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

#[tokio::test]
async fn create_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // First create a plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Test Plan".to_string(),
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
            customer_id: "22222222-2222-2222-2222-222222222222".to_string(),
            plan_id: plan.plan_id.clone(),
            billing_anchor_day: 15,
            start_date: "2026-01-15".to_string(),
            trial_end_date: "".to_string(),
            proration_mode: 1, // Immediate
            metadata: "".to_string(),
        },
    );

    let response = client.create_subscription(request).await;
    assert!(
        response.is_ok(),
        "CreateSubscription should succeed: {:?}",
        response.err()
    );

    let resp = response.unwrap().into_inner();
    let subscription = resp.subscription.unwrap();
    assert_eq!(subscription.plan_id, plan.plan_id);
    assert_eq!(subscription.billing_anchor_day, 15);
    assert!(!subscription.subscription_id.is_empty());

    // Should also create initial billing cycle
    assert!(resp.initial_cycle.is_some());

    app.cleanup().await;
}

#[tokio::test]
async fn create_subscription_with_trial_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Trial Plan".to_string(),
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

    // Create subscription with trial
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "33333333-3333-3333-3333-333333333333".to_string(),
            plan_id: plan.plan_id,
            billing_anchor_day: 1,
            start_date: "2026-01-01".to_string(),
            trial_end_date: "2026-01-31".to_string(),
            proration_mode: 1,
            metadata: "".to_string(),
        },
    );

    let response = client.create_subscription(request).await;
    assert!(response.is_ok());

    let subscription = response.unwrap().into_inner().subscription.unwrap();
    assert_eq!(subscription.trial_end_date, "2026-01-31");

    app.cleanup().await;
}

#[tokio::test]
async fn get_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create plan and subscription
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Get Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "75.00".to_string(),
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

    let sub_request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "44444444-4444-4444-4444-444444444444".to_string(),
            plan_id: plan.plan_id.clone(),
            billing_anchor_day: 10,
            start_date: "".to_string(),
            trial_end_date: "".to_string(),
            proration_mode: 0,
            metadata: "".to_string(),
        },
    );
    let created = client
        .create_subscription(sub_request)
        .await
        .unwrap()
        .into_inner()
        .subscription
        .unwrap();

    // Get subscription
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: created.subscription_id.clone(),
        },
    );

    let response = client.get_subscription(get_request).await;
    assert!(response.is_ok());

    let resp = response.unwrap().into_inner();
    let subscription = resp.subscription.unwrap();
    assert_eq!(subscription.subscription_id, created.subscription_id);
    assert_eq!(subscription.plan_id, plan.plan_id);

    // Should include current cycle
    assert!(resp.current_cycle.is_some());

    app.cleanup().await;
}

#[tokio::test]
async fn list_subscriptions_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "List Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "25.00".to_string(),
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

    // Create multiple subscriptions
    for i in 1..=3 {
        let request = with_tenant(
            TEST_TENANT_ID,
            CreateSubscriptionRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                customer_id: format!("5555555{}-5555-5555-5555-555555555555", i),
                plan_id: plan.plan_id.clone(),
                billing_anchor_day: i,
                start_date: "".to_string(),
                trial_end_date: "".to_string(),
                proration_mode: 0,
                metadata: "".to_string(),
            },
        );
        client.create_subscription(request).await.unwrap();
    }

    // List subscriptions
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListSubscriptionsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "".to_string(),
            status: 0, // All
            plan_id: "".to_string(),
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_subscriptions(list_request).await;
    assert!(response.is_ok());

    let subscriptions = response.unwrap().into_inner().subscriptions;
    assert_eq!(subscriptions.len(), 3);

    app.cleanup().await;
}

#[tokio::test]
async fn list_subscriptions_filters_by_customer() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Filter Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "30.00".to_string(),
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

    let customer_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let customer_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    // Create subscriptions for different customers
    for (i, customer) in [(1, customer_a), (2, customer_a), (3, customer_b)].iter() {
        let request = with_tenant(
            TEST_TENANT_ID,
            CreateSubscriptionRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                customer_id: customer.to_string(),
                plan_id: plan.plan_id.clone(),
                billing_anchor_day: *i,
                start_date: "".to_string(),
                trial_end_date: "".to_string(),
                proration_mode: 0,
                metadata: "".to_string(),
            },
        );
        client.create_subscription(request).await.unwrap();
    }

    // Filter by customer_a
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListSubscriptionsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: customer_a.to_string(),
            status: 0,
            plan_id: "".to_string(),
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_subscriptions(list_request).await;
    assert!(response.is_ok());

    let subscriptions = response.unwrap().into_inner().subscriptions;
    assert_eq!(subscriptions.len(), 2);

    app.cleanup().await;
}
