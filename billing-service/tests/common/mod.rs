//! Test helper module for billing-service integration tests.
//!
//! Provides common setup utilities for PostgreSQL-based tests.

#![allow(dead_code)]

use billing_service::config::{AuthConfig, BillingConfig, DatabaseConfig, InvoicingServiceConfig};
use billing_service::services::{init_metrics, Database};
use billing_service::startup::Application;
use service_core::config::Config as CoreConfig;
use std::sync::atomic::{AtomicU32, Ordering};
use uuid::Uuid;

// Test constants for tenant context
pub const TEST_TENANT_ID: &str = "11111111-1111-1111-1111-111111111111";
pub const TEST_CUSTOMER_ID: &str = "22222222-2222-2222-2222-222222222222";

// Counter for unique schema names
static SCHEMA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Get the database URL for testing from environment or use default.
pub fn get_test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:pass%40word1@localhost:5432/micros_test".to_string()
    })
}

/// Generate a unique schema name for test isolation.
fn unique_schema_name() -> String {
    let counter = SCHEMA_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_billing_{}_{}", std::process::id(), counter)
}

/// Test application wrapper for integration tests.
pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub http_port: u16,
    pub grpc_port: u16,
    pub db: Database,
    schema_name: String,
}

impl TestApp {
    /// Spawn a new test application on random ports.
    pub async fn spawn() -> Self {
        // Initialize metrics (required for metrics endpoint test)
        init_metrics();

        let base_url = get_test_database_url();
        let schema_name = unique_schema_name();

        // Create schema for test isolation
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&base_url)
            .await
            .expect("Failed to connect to test database");

        // Create schema and set search path
        sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name))
            .execute(&pool)
            .await
            .ok();
        sqlx::query(&format!("CREATE SCHEMA {}", schema_name))
            .execute(&pool)
            .await
            .expect("Failed to create test schema");

        // Close the setup pool
        pool.close().await;

        // Create config with schema in search path
        // Use ? or & depending on whether URL already has query parameters
        let separator = if base_url.contains('?') { "&" } else { "?" };
        let db_url_with_schema = format!(
            "{}{}options=-c search_path%3D{}",
            base_url, separator, schema_name
        );

        let config = BillingConfig {
            common: CoreConfig { port: 0 }, // Random port
            service_name: "billing-service-test".to_string(),
            service_version: "0.1.0".to_string(),
            log_level: "warn".to_string(),
            otlp_endpoint: None,
            database: DatabaseConfig {
                url: db_url_with_schema,
                max_connections: 5,
                min_connections: 1,
            },
            invoicing_service: InvoicingServiceConfig {
                url: "http://localhost:50053".to_string(), // May not be available in tests
            },
            auth: AuthConfig {
                auth_service_endpoint: "http://localhost:3001".to_string(),
            },
        };

        let app = Application::build(config)
            .await
            .expect("Failed to build test application");

        let http_port = app.http_port();
        let grpc_port = app.grpc_port();
        let db = Database::new(&get_test_database_url(), 5, 1)
            .await
            .expect("Failed to create test database");

        let http_address = format!("http://127.0.0.1:{}", http_port);
        let grpc_address = format!("http://127.0.0.1:{}", grpc_port);

        tokio::spawn(async move {
            app.run_until_stopped().await.ok();
        });

        // Wait for HTTP server to be ready by polling health endpoint
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/health", http_port);
        for _ in 0..50 {
            if client.get(&health_url).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Additional wait for gRPC server
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        TestApp {
            http_address,
            grpc_address,
            http_port,
            grpc_port,
            db,
            schema_name,
        }
    }

    /// Create a gRPC client connected to this test app.
    pub async fn grpc_client(
        &self,
    ) -> billing_service::grpc::proto::billing_service_client::BillingServiceClient<
        tonic::transport::Channel,
    > {
        billing_service::grpc::proto::billing_service_client::BillingServiceClient::connect(
            self.grpc_address.clone(),
        )
        .await
        .expect("Failed to connect to gRPC server")
    }

    /// Get test tenant ID.
    pub fn tenant_id(&self) -> Uuid {
        Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    /// Get test customer ID.
    pub fn customer_id(&self) -> Uuid {
        Uuid::parse_str(TEST_CUSTOMER_ID).unwrap()
    }

    /// Cleanup test resources (schema).
    pub async fn cleanup(&self) {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&get_test_database_url())
            .await
            .ok();

        if let Some(pool) = pool {
            let _ = sqlx::query(&format!(
                "DROP SCHEMA IF EXISTS {} CASCADE",
                self.schema_name
            ))
            .execute(&pool)
            .await;
            pool.close().await;
        }
    }
}

/// Helper to create metadata with tenant_id for gRPC requests.
pub fn create_metadata(tenant_id: &str) -> tonic::metadata::MetadataMap {
    let mut metadata = tonic::metadata::MetadataMap::new();
    metadata.insert("x-tenant-id", tenant_id.parse().unwrap());
    metadata
}

/// Helper to create a request with tenant metadata.
pub fn with_tenant<T>(tenant_id: &str, request: T) -> tonic::Request<T> {
    let mut req = tonic::Request::new(request);
    req.metadata_mut()
        .insert("x-tenant-id", tenant_id.parse().unwrap());
    req
}
