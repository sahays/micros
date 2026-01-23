//! Usage tracking integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

/// Helper to create a subscription with usage components.
async fn create_subscription_with_usage(
    client: &mut billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    >,
) -> (Subscription, BillingPlan) {
    // Create plan with usage components
    let plan_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Usage Plan".to_string(),
            description: "".to_string(),
            billing_interval: 3,
            interval_count: 1,
            base_price: "0.00".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![
                CreateUsageComponentInput {
                    name: "API Calls".to_string(),
                    unit_name: "calls".to_string(),
                    unit_price: "0.001".to_string(),
                    included_units: 1000,
                },
                CreateUsageComponentInput {
                    name: "Storage".to_string(),
                    unit_name: "GB".to_string(),
                    unit_price: "0.10".to_string(),
                    included_units: 10,
                },
            ],
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
            customer_id: "77777777-7777-7777-7777-777777777777".to_string(),
            plan_id: plan.plan_id.clone(),
            billing_anchor_day: 1,
            start_date: "".to_string(),
            trial_end_date: "".to_string(),
            proration_mode: 1,
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

    (subscription, plan)
}

#[tokio::test]
async fn record_usage_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, plan) = create_subscription_with_usage(&mut client).await;
    let component_id = plan.usage_components[0].component_id.clone();

    // Record usage
    let request = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            component_id: component_id.clone(),
            quantity: "500".to_string(),
            timestamp: None,
            idempotency_key: "test-usage-001".to_string(),
            metadata: "".to_string(),
        },
    );

    let response = client.record_usage(request).await;
    assert!(
        response.is_ok(),
        "RecordUsage should succeed: {:?}",
        response.err()
    );

    let record = response.unwrap().into_inner().usage_record.unwrap();
    assert_eq!(record.subscription_id, subscription.subscription_id);
    assert_eq!(record.component_id, component_id);
    assert_eq!(record.quantity, "500.0000");
    assert_eq!(record.idempotency_key, "test-usage-001");
    assert!(!record.is_invoiced);

    app.cleanup().await;
}

#[tokio::test]
async fn record_usage_with_idempotency_key_deduplicates() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, plan) = create_subscription_with_usage(&mut client).await;
    let component_id = plan.usage_components[0].component_id.clone();

    // Record first usage
    let request1 = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            component_id: component_id.clone(),
            quantity: "100".to_string(),
            timestamp: None,
            idempotency_key: "idempotent-key-123".to_string(),
            metadata: "".to_string(),
        },
    );
    let first = client
        .record_usage(request1)
        .await
        .unwrap()
        .into_inner()
        .usage_record
        .unwrap();

    // Record same usage with same key (should return existing record)
    let request2 = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            component_id,
            quantity: "100".to_string(),
            timestamp: None,
            idempotency_key: "idempotent-key-123".to_string(),
            metadata: "".to_string(),
        },
    );
    let second = client
        .record_usage(request2)
        .await
        .unwrap()
        .into_inner()
        .usage_record
        .unwrap();

    // Should be the same record
    assert_eq!(first.record_id, second.record_id);

    app.cleanup().await;
}

#[tokio::test]
async fn get_usage_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, plan) = create_subscription_with_usage(&mut client).await;
    let component_id = plan.usage_components[0].component_id.clone();

    // Record usage
    let record_request = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            component_id,
            quantity: "250".to_string(),
            timestamp: None,
            idempotency_key: "get-usage-test".to_string(),
            metadata: "".to_string(),
        },
    );
    let recorded = client
        .record_usage(record_request)
        .await
        .unwrap()
        .into_inner()
        .usage_record
        .unwrap();

    // Get usage
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            record_id: recorded.record_id.clone(),
        },
    );

    let response = client.get_usage(get_request).await;
    assert!(response.is_ok());

    let fetched = response.unwrap().into_inner().usage_record.unwrap();
    assert_eq!(fetched.record_id, recorded.record_id);
    assert_eq!(fetched.quantity, "250.0000");

    app.cleanup().await;
}

#[tokio::test]
async fn list_usage_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, plan) = create_subscription_with_usage(&mut client).await;
    let component_id = plan.usage_components[0].component_id.clone();

    // Record multiple usage records
    for i in 1..=5 {
        let request = with_tenant(
            TEST_TENANT_ID,
            RecordUsageRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                subscription_id: subscription.subscription_id.clone(),
                component_id: component_id.clone(),
                quantity: format!("{}", i * 100),
                timestamp: None,
                idempotency_key: format!("list-usage-{}", i),
                metadata: "".to_string(),
            },
        );
        client.record_usage(request).await.unwrap();
    }

    // List usage
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            component_id: "".to_string(),
            cycle_id: "".to_string(),
            is_invoiced: false,
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let response = client.list_usage(list_request).await;
    assert!(response.is_ok());

    let records = response.unwrap().into_inner().usage_records;
    assert_eq!(records.len(), 5);

    app.cleanup().await;
}

#[tokio::test]
async fn get_usage_summary_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let (subscription, plan) = create_subscription_with_usage(&mut client).await;
    let api_component_id = plan.usage_components[0].component_id.clone();
    let storage_component_id = plan.usage_components[1].component_id.clone();

    // Record API usage (1500 calls, 1000 included = 500 billable)
    let api_request = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            component_id: api_component_id,
            quantity: "1500".to_string(),
            timestamp: None,
            idempotency_key: "summary-api-usage".to_string(),
            metadata: "".to_string(),
        },
    );
    client.record_usage(api_request).await.unwrap();

    // Record storage usage (15 GB, 10 included = 5 billable)
    let storage_request = with_tenant(
        TEST_TENANT_ID,
        RecordUsageRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            component_id: storage_component_id,
            quantity: "15".to_string(),
            timestamp: None,
            idempotency_key: "summary-storage-usage".to_string(),
            metadata: "".to_string(),
        },
    );
    client.record_usage(storage_request).await.unwrap();

    // Get usage summary
    let summary_request = with_tenant(
        TEST_TENANT_ID,
        GetUsageSummaryRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            subscription_id: subscription.subscription_id,
            cycle_id: "".to_string(), // Current cycle
        },
    );

    let response = client.get_usage_summary(summary_request).await;
    assert!(response.is_ok());

    let summaries = response.unwrap().into_inner().component_summaries;
    assert_eq!(summaries.len(), 2);

    // Find API calls summary
    let api_summary = summaries.iter().find(|s| s.name == "API Calls").unwrap();
    assert_eq!(api_summary.total_quantity, "1500.0000");
    assert_eq!(api_summary.included_units, 1000);
    assert_eq!(api_summary.billable_units, "500.0000");
    // Amount should be 500 * 0.001 = 0.5 (8 decimal places from multiplication)
    assert_eq!(api_summary.amount, "0.50000000");

    // Find storage summary
    let storage_summary = summaries.iter().find(|s| s.name == "Storage").unwrap();
    assert_eq!(storage_summary.total_quantity, "15.0000");
    assert_eq!(storage_summary.included_units, 10);
    assert_eq!(storage_summary.billable_units, "5.0000");
    // Amount should be 5 * 0.10 = 0.5 (8 decimal places from multiplication)
    assert_eq!(storage_summary.amount, "0.50000000");

    app.cleanup().await;
}
