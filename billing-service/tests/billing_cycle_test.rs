//! Billing cycle integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create a plan and subscription.
async fn create_test_subscription(
    client: &mut billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    >,
) -> (Subscription, String) {
    // Create plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Cycle Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "99.00".to_string(),
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
    let sub_request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "88888888-8888-8888-8888-888888888888".to_string(),
            plan_id: plan.plan_id.clone(),
            billing_anchor_day: 15,
            start_date: "".to_string(),
            trial_end_date: "".to_string(),
            proration_mode: 1,
            metadata: "".to_string(),
        },
    );
    let resp = client
        .create_subscription(sub_request)
        .await
        .unwrap()
        .into_inner();
    let subscription = resp.subscription.unwrap();
    let initial_cycle_id = resp.initial_cycle.unwrap().cycle_id;

    (subscription, initial_cycle_id)
}

#[tokio::test]
async fn get_billing_cycle_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (_subscription, cycle_id) = create_test_subscription(&mut client).await;

    // Get billing cycle
    let request = with_tenant(
        TEST_TENANT_ID,
        GetBillingCycleRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            cycle_id: cycle_id.clone(),
        },
    );

    let response = client.get_billing_cycle(request).await;
    assert!(
        response.is_ok(),
        "GetBillingCycle should succeed: {:?}",
        response.err()
    );

    let cycle = response.unwrap().into_inner().billing_cycle.unwrap();
    assert_eq!(cycle.cycle_id, cycle_id);
    assert_eq!(cycle.status, 1); // PENDING

    app.cleanup().await;
}

#[tokio::test]
async fn get_billing_cycle_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        GetBillingCycleRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            cycle_id: "99999999-9999-9999-9999-999999999999".to_string(),
        },
    );

    let response = client.get_billing_cycle(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_billing_cycles_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _) = create_test_subscription(&mut client).await;

    // List billing cycles
    let request = with_tenant(
        TEST_TENANT_ID,
        ListBillingCyclesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            status: 0, // All
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_billing_cycles(request).await;
    assert!(response.is_ok());

    let cycles = response.unwrap().into_inner().billing_cycles;
    assert!(!cycles.is_empty());
    assert_eq!(cycles[0].status, 1); // PENDING

    app.cleanup().await;
}

#[tokio::test]
async fn advance_billing_cycle_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _) = create_test_subscription(&mut client).await;

    // Advance billing cycle
    let request = with_tenant(
        TEST_TENANT_ID,
        AdvanceBillingCycleRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
        },
    );

    let response = client.advance_billing_cycle(request).await;
    assert!(
        response.is_ok(),
        "AdvanceBillingCycle should succeed: {:?}",
        response.err()
    );

    let resp = response.unwrap().into_inner();
    assert!(resp.previous_cycle.is_some());
    assert!(resp.new_cycle.is_some());

    let previous = resp.previous_cycle.unwrap();
    let new_cycle = resp.new_cycle.unwrap();

    // Previous cycle should now be closed
    assert_ne!(previous.cycle_id, new_cycle.cycle_id);

    // Verify new cycle was created
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListBillingCyclesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            status: 0,
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let cycles = client
        .list_billing_cycles(list_request)
        .await
        .unwrap()
        .into_inner()
        .billing_cycles;
    assert_eq!(cycles.len(), 2);

    app.cleanup().await;
}
