//! Billing run integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create a subscription.
async fn create_test_subscription(
    client: &mut billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    >,
    plan_name: &str,
) -> Subscription {
    // Create plan
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: plan_name.to_string(),
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
    let sub_request = with_tenant(
        TEST_TENANT_ID,
        CreateSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string(),
            plan_id: plan.plan_id,
            billing_anchor_day: 1,
            start_date: "2025-01-01".to_string(), // Past date to make it due
            trial_end_date: "".to_string(),
            proration_mode: 1,
            metadata: "".to_string(),
        },
    );
    client
        .create_subscription(sub_request)
        .await
        .unwrap()
        .into_inner()
        .subscription
        .unwrap()
}

#[tokio::test]
async fn run_billing_for_subscription_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let subscription = create_test_subscription(&mut client, "Billing Run Plan").await;

    // Run billing for single subscription
    let request = with_tenant(
        TEST_TENANT_ID,
        RunBillingForSubscriptionRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
        },
    );

    let response = client.run_billing_for_subscription(request).await;
    assert!(
        response.is_ok(),
        "RunBillingForSubscription should succeed: {:?}",
        response.err()
    );

    let result = response.unwrap().into_inner().result.unwrap();
    assert_eq!(result.status, "success");

    app.cleanup().await;
}

#[tokio::test]
async fn run_billing_batch_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a subscription (may not be due, but tests the batch logic)
    let _subscription = create_test_subscription(&mut client, "Batch Plan").await;

    // Run batch billing
    let request = with_tenant(
        TEST_TENANT_ID,
        RunBillingRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            run_type: 2, // MANUAL
        },
    );

    let response = client.run_billing(request).await;
    assert!(
        response.is_ok(),
        "RunBilling should succeed: {:?}",
        response.err()
    );

    let billing_run = response.unwrap().into_inner().billing_run.unwrap();
    assert!(!billing_run.run_id.is_empty());
    assert!(billing_run.status == 2 || billing_run.status == 3); // COMPLETED or FAILED

    app.cleanup().await;
}

#[tokio::test]
async fn get_billing_run_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let _subscription = create_test_subscription(&mut client, "Get Run Plan").await;

    // Run billing first
    let run_request = with_tenant(
        TEST_TENANT_ID,
        RunBillingRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            run_type: 2, // MANUAL
        },
    );
    let created = client
        .run_billing(run_request)
        .await
        .unwrap()
        .into_inner()
        .billing_run
        .unwrap();

    // Get billing run
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetBillingRunRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            run_id: created.run_id.clone(),
        },
    );

    let response = client.get_billing_run(get_request).await;
    assert!(response.is_ok());

    let billing_run = response.unwrap().into_inner().billing_run.unwrap();
    assert_eq!(billing_run.run_id, created.run_id);

    app.cleanup().await;
}

#[tokio::test]
async fn get_billing_run_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        GetBillingRunRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            run_id: "00000000-0000-0000-0000-000000000000".to_string(),
        },
    );

    let response = client.get_billing_run(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_billing_runs_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let _subscription = create_test_subscription(&mut client, "List Runs Plan").await;

    // Run multiple billings
    for _ in 0..2 {
        let request = with_tenant(
            TEST_TENANT_ID,
            RunBillingRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                run_type: 2, // MANUAL
            },
        );
        client.run_billing(request).await.unwrap();
    }

    // List billing runs
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListBillingRunsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            status: 0,   // All
            run_type: 0, // All
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_billing_runs(list_request).await;
    assert!(response.is_ok());

    let runs = response.unwrap().into_inner().billing_runs;
    assert!(runs.len() >= 2);

    app.cleanup().await;
}

#[tokio::test]
async fn list_billing_runs_filters_by_status() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let _subscription = create_test_subscription(&mut client, "Filter Runs Plan").await;

    // Run billing
    let run_request = with_tenant(
        TEST_TENANT_ID,
        RunBillingRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            run_type: 2, // MANUAL
        },
    );
    client.run_billing(run_request).await.unwrap();

    // Filter by COMPLETED status
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListBillingRunsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            status: 2, // COMPLETED
            run_type: 0,
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_billing_runs(list_request).await;
    assert!(response.is_ok());

    let runs = response.unwrap().into_inner().billing_runs;
    for run in runs {
        assert_eq!(run.status, 2); // All COMPLETED
    }

    app.cleanup().await;
}
