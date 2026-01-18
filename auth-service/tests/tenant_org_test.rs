//! Tenant and Organization integration tests for auth-service v2.
//!
//! Tests tenant management and org node hierarchy operations.

mod common;

use common::{cleanup_test_data, TestApp};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn create_org_node_succeeds() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    // Create tenant directly in DB
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, 'Test Tenant', 'active', now())
        "#,
    )
    .bind(tenant_id)
    .execute(&app.pool)
    .await
    .expect("Failed to create tenant");

    let client = app.client();

    // Act - Create org node
    let request_body = json!({
        "tenant_id": tenant_id,
        "node_type_code": "region",
        "node_label": "North Region"
    });

    let response = client
        .post(format!("{}/orgs", app.address))
        .json(&request_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("org_node_id").is_some());
    assert_eq!(body["node_label"], "North Region");
    assert_eq!(body["node_type_code"], "region");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn create_child_org_node_succeeds() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    // Create tenant
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, 'Test Tenant', 'active', now())
        "#,
    )
    .bind(tenant_id)
    .execute(&app.pool)
    .await
    .expect("Failed to create tenant");

    let client = app.client();

    // Create parent org node
    let parent_body = json!({
        "tenant_id": tenant_id,
        "node_type_code": "region",
        "node_label": "North Region"
    });

    let parent_response = client
        .post(format!("{}/orgs", app.address))
        .json(&parent_body)
        .send()
        .await
        .expect("Failed to create parent org");

    let parent_body: serde_json::Value = parent_response.json().await.unwrap();
    let parent_id = parent_body["org_node_id"].as_str().unwrap();

    // Act - Create child org node
    let child_body = json!({
        "tenant_id": tenant_id,
        "node_type_code": "district",
        "node_label": "District 1",
        "parent_org_node_id": parent_id
    });

    let response = client
        .post(format!("{}/orgs", app.address))
        .json(&child_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body.get("org_node_id").is_some());
    assert_eq!(body["parent_org_node_id"], parent_id);
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn get_org_node_returns_node() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    // Create tenant
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, 'Test Tenant', 'active', now())
        "#,
    )
    .bind(tenant_id)
    .execute(&app.pool)
    .await
    .expect("Failed to create tenant");

    let client = app.client();

    // Create org node
    let create_body = json!({
        "tenant_id": tenant_id,
        "node_type_code": "region",
        "node_label": "Test Region"
    });

    let create_response = client
        .post(format!("{}/orgs", app.address))
        .json(&create_body)
        .send()
        .await
        .expect("Failed to create org");

    let created: serde_json::Value = create_response.json().await.unwrap();
    let org_node_id = created["org_node_id"].as_str().unwrap();

    // Act - Get org node
    let response = client
        .get(format!("{}/orgs/{}", app.address, org_node_id))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["org_node_id"], org_node_id);
    assert_eq!(body["node_label"], "Test Region");
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn list_tenant_org_nodes_returns_all_nodes() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    // Create tenant
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, 'Test Tenant', 'active', now())
        "#,
    )
    .bind(tenant_id)
    .execute(&app.pool)
    .await
    .expect("Failed to create tenant");

    let client = app.client();

    // Create multiple org nodes
    for i in 1..=3 {
        let body = json!({
            "tenant_id": tenant_id,
            "node_type_code": "region",
            "node_label": format!("Region {}", i)
        });

        client
            .post(format!("{}/orgs", app.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to create org");
    }

    // Act - List tenant org nodes
    let response = client
        .get(format!("{}/tenants/{}/orgs", app.address, tenant_id))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: Vec<serde_json::Value> = response.json().await.expect("Failed to parse response");
    assert_eq!(body.len(), 3);
}

#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn get_org_descendants_returns_subtree() {
    // Arrange
    let app = TestApp::spawn().await.expect("Failed to spawn test app");
    cleanup_test_data(&app.pool)
        .await
        .expect("Failed to cleanup");

    // Create tenant
    let tenant_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (tenant_id, tenant_label, tenant_state_code, created_utc)
        VALUES ($1, 'Test Tenant', 'active', now())
        "#,
    )
    .bind(tenant_id)
    .execute(&app.pool)
    .await
    .expect("Failed to create tenant");

    let client = app.client();

    // Create parent
    let parent_body = json!({
        "tenant_id": tenant_id,
        "node_type_code": "region",
        "node_label": "Parent Region"
    });

    let parent_response = client
        .post(format!("{}/orgs", app.address))
        .json(&parent_body)
        .send()
        .await
        .expect("Failed to create parent");

    let parent: serde_json::Value = parent_response.json().await.unwrap();
    let parent_id = parent["org_node_id"].as_str().unwrap();

    // Create children
    for i in 1..=2 {
        let child_body = json!({
            "tenant_id": tenant_id,
            "node_type_code": "district",
            "node_label": format!("Child {}", i),
            "parent_org_node_id": parent_id
        });

        client
            .post(format!("{}/orgs", app.address))
            .json(&child_body)
            .send()
            .await
            .expect("Failed to create child");
    }

    // Act - Get descendants
    let response = client
        .get(format!("{}/orgs/{}/descendants", app.address, parent_id))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(response.status(), 200);

    let body: Vec<serde_json::Value> = response.json().await.expect("Failed to parse response");
    // Should include the parent itself plus 2 children
    assert_eq!(body.len(), 3);
}
