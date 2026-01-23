//! Charge management integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create a subscription with its billing cycle.
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
            name: "Charge Test Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
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
    let sub_request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "99999999-9999-9999-9999-999999999999".to_string(),
            plan_id: plan.plan_id,
            billing_anchor_day: 1,
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
    let cycle_id = resp.initial_cycle.unwrap().cycle_id;

    (subscription, cycle_id)
}

#[tokio::test]
async fn create_one_time_charge_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _cycle_id) = create_test_subscription(&mut client).await;

    // Create one-time charge
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateOneTimeChargeRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            description: "Setup fee".to_string(),
            amount: "25.00".to_string(),
            metadata: "".to_string(),
        },
    );

    let response = client.create_one_time_charge(request).await;
    assert!(
        response.is_ok(),
        "CreateOneTimeCharge should succeed: {:?}",
        response.err()
    );

    let charge = response.unwrap().into_inner().charge.unwrap();
    assert_eq!(charge.description, "Setup fee");
    assert_eq!(charge.amount, "25.0000");
    assert_eq!(charge.charge_type, 3); // ONE_TIME

    app.cleanup().await;
}

#[tokio::test]
async fn create_one_time_charge_with_credit_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _) = create_test_subscription(&mut client).await;

    // Create credit (negative charge)
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateOneTimeChargeRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            description: "Promotional credit".to_string(),
            amount: "-10.00".to_string(),
            metadata: "".to_string(),
        },
    );

    let response = client.create_one_time_charge(request).await;
    assert!(response.is_ok());

    let charge = response.unwrap().into_inner().charge.unwrap();
    assert_eq!(charge.amount, "-10.0000");

    app.cleanup().await;
}

#[tokio::test]
async fn get_charge_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, _) = create_test_subscription(&mut client).await;

    // Create a charge first
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateOneTimeChargeRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            description: "Test charge".to_string(),
            amount: "15.00".to_string(),
            metadata: "".to_string(),
        },
    );
    let created = client
        .create_one_time_charge(create_request)
        .await
        .unwrap()
        .into_inner()
        .charge
        .unwrap();

    // Get charge
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetChargeRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            charge_id: created.charge_id.clone(),
        },
    );

    let response = client.get_charge(get_request).await;
    assert!(response.is_ok());

    let charge = response.unwrap().into_inner().charge.unwrap();
    assert_eq!(charge.charge_id, created.charge_id);
    assert_eq!(charge.amount, "15.0000");

    app.cleanup().await;
}

#[tokio::test]
async fn get_charge_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        GetChargeRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            charge_id: "00000000-0000-0000-0000-000000000000".to_string(),
        },
    );

    let response = client.get_charge(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_charges_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, cycle_id) = create_test_subscription(&mut client).await;

    // Create multiple charges
    for i in 1..=3 {
        let request = with_tenant(
            TEST_TENANT_ID,
            CreateOneTimeChargeRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                subscription_id: subscription.subscription_id.clone(),
                description: format!("Charge {}", i),
                amount: format!("{}.00", i * 10),
                metadata: "".to_string(),
            },
        );
        client.create_one_time_charge(request).await.unwrap();
    }

    // List charges
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListChargesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            cycle_id,
            charge_type: 0, // All
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_charges(list_request).await;
    assert!(response.is_ok());

    let charges = response.unwrap().into_inner().charges;
    assert_eq!(charges.len(), 3);

    app.cleanup().await;
}

#[tokio::test]
async fn list_charges_filters_by_type() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, cycle_id) = create_test_subscription(&mut client).await;

    // Create one-time charges
    for i in 1..=2 {
        let request = with_tenant(
            TEST_TENANT_ID,
            CreateOneTimeChargeRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                subscription_id: subscription.subscription_id.clone(),
                description: format!("One-time {}", i),
                amount: "5.00".to_string(),
                metadata: "".to_string(),
            },
        );
        client.create_one_time_charge(request).await.unwrap();
    }

    // Filter by ONE_TIME type
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListChargesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            cycle_id,
            charge_type: 3, // ONE_TIME
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_charges(list_request).await;
    assert!(response.is_ok());

    let charges = response.unwrap().into_inner().charges;
    assert_eq!(charges.len(), 2);
    for charge in charges {
        assert_eq!(charge.charge_type, 3); // All ONE_TIME
    }

    app.cleanup().await;
}
