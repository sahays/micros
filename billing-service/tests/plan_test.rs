//! Plan management integration tests for billing-service.

mod common;

use billing_service::grpc::proto::*;
use common::{with_tenant, TestApp, TEST_TENANT_ID};

#[tokio::test]
async fn create_plan_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Basic Plan".to_string(),
            description: "A basic billing plan".to_string(),
            billing_interval: 3, // Monthly
            interval_count: 1,
            base_price: "29.99".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );

    let response = client.create_plan(request).await;
    assert!(
        response.is_ok(),
        "CreatePlan should succeed: {:?}",
        response.err()
    );

    let plan = response.unwrap().into_inner().plan.unwrap();
    assert_eq!(plan.name, "Basic Plan");
    assert_eq!(plan.base_price, "29.9900");
    assert_eq!(plan.currency, "USD");
    assert!(!plan.plan_id.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn get_plan_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // First create a plan
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Premium Plan".to_string(),
            description: "A premium billing plan".to_string(),
            billing_interval: 3, // Monthly
            interval_count: 1,
            base_price: "99.99".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );

    let create_response = client.create_plan(create_request).await.unwrap();
    let created_plan = create_response.into_inner().plan.unwrap();

    // Now get the plan
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetPlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            plan_id: created_plan.plan_id.clone(),
        },
    );

    let get_response = client.get_plan(get_request).await;
    assert!(
        get_response.is_ok(),
        "GetPlan should succeed: {:?}",
        get_response.err()
    );

    let plan = get_response.unwrap().into_inner().plan.unwrap();
    assert_eq!(plan.plan_id, created_plan.plan_id);
    assert_eq!(plan.name, "Premium Plan");

    app.cleanup().await;
}

#[tokio::test]
async fn list_plans_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a couple of plans
    for i in 1..=3 {
        let request = with_tenant(
            TEST_TENANT_ID,
            CreatePlanRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                name: format!("Plan {}", i),
                description: "".to_string(),
                billing_interval: 3, // Monthly
                interval_count: 1,
                base_price: format!("{}.00", i * 10),
                currency: "USD".to_string(),
                tax_rate_id: "".to_string(),
                usage_components: vec![],
                metadata: "".to_string(),
            },
        );
        client.create_plan(request).await.unwrap();
    }

    // List plans
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListPlansRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            include_archived: false,
            page_size: 10,
            page_token: "".to_string(),
        },
    );

    let list_response = client.list_plans(list_request).await;
    assert!(
        list_response.is_ok(),
        "ListPlans should succeed: {:?}",
        list_response.err()
    );

    let plans = list_response.unwrap().into_inner().plans;
    assert_eq!(plans.len(), 3);

    app.cleanup().await;
}

#[tokio::test]
async fn update_plan_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a plan
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Original Name".to_string(),
            description: "Original description".to_string(),
            billing_interval: 3, // Monthly
            interval_count: 1,
            base_price: "50.00".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![],
            metadata: "".to_string(),
        },
    );

    let created = client
        .create_plan(create_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();

    // Update the plan
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            plan_id: created.plan_id.clone(),
            name: "Updated Name".to_string(),
            description: "Updated description".to_string(),
            base_price: "55.00".to_string(),
            tax_rate_id: "".to_string(),
            metadata: "".to_string(),
        },
    );

    let update_response = client.update_plan(update_request).await;
    assert!(
        update_response.is_ok(),
        "UpdatePlan should succeed: {:?}",
        update_response.err()
    );

    let updated = update_response.unwrap().into_inner().plan.unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description, "Updated description");
    assert_eq!(updated.base_price, "55.0000");

    app.cleanup().await;
}

#[tokio::test]
async fn archive_plan_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a plan
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "To Be Archived".to_string(),
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

    let created = client
        .create_plan(create_request)
        .await
        .unwrap()
        .into_inner()
        .plan
        .unwrap();
    assert!(!created.is_archived);

    // Archive the plan
    let archive_request = with_tenant(
        TEST_TENANT_ID,
        ArchivePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            plan_id: created.plan_id.clone(),
        },
    );

    let archive_response = client.archive_plan(archive_request).await;
    assert!(
        archive_response.is_ok(),
        "ArchivePlan should succeed: {:?}",
        archive_response.err()
    );

    let archived = archive_response.unwrap().into_inner().plan.unwrap();
    assert!(archived.is_archived);

    app.cleanup().await;
}

#[tokio::test]
async fn create_plan_with_usage_components_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        CreatePlanRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Usage Plan".to_string(),
            description: "Plan with usage components".to_string(),
            billing_interval: 3, // Monthly
            interval_count: 1,
            base_price: "0.00".to_string(),
            currency: "USD".to_string(),
            tax_rate_id: "".to_string(),
            usage_components: vec![
                CreateUsageComponentInput {
                    name: "API Calls".to_string(),
                    unit_name: "calls".to_string(),
                    unit_price: "0.001".to_string(),
                    included_units: 10000,
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

    let response = client.create_plan(request).await;
    assert!(
        response.is_ok(),
        "CreatePlan with usage components should succeed: {:?}",
        response.err()
    );

    let plan = response.unwrap().into_inner().plan.unwrap();
    assert_eq!(plan.name, "Usage Plan");
    assert_eq!(plan.usage_components.len(), 2);
    assert_eq!(plan.usage_components[0].name, "API Calls");
    assert_eq!(plan.usage_components[1].name, "Storage");

    app.cleanup().await;
}
