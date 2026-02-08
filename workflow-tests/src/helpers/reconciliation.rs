#![allow(dead_code)]

use crate::proto::reconciliation::reconciliation_service_client::ReconciliationServiceClient;
use std::sync::Once;
use tonic::transport::Channel;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize tracing for tests (only once).
pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,reconciliation_service=debug")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

fn grpc_endpoint() -> String {
    std::env::var("RECONCILIATION_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:50058".to_string())
}

/// Test application wrapper.
pub struct TestApp {
    pub grpc_client: ReconciliationServiceClient<Channel>,
    pub tenant_id: Uuid,
}

/// Connect to the deployed reconciliation-service and return the gRPC client with a unique tenant ID.
pub async fn spawn_app() -> TestApp {
    init_tracing();

    let endpoint = grpc_endpoint();
    let grpc_client = ReconciliationServiceClient::connect(endpoint.clone())
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Failed to connect to reconciliation-service at {}: {}",
                endpoint, e
            )
        });

    let tenant_id = Uuid::new_v4();
    TestApp {
        grpc_client,
        tenant_id,
    }
}

/// Helper to inject tenant ID and user ID into request metadata.
pub fn with_tenant<T>(request: T, tenant_id: &Uuid) -> tonic::Request<T> {
    let mut req = tonic::Request::new(request);
    req.metadata_mut()
        .insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    req.metadata_mut()
        .insert("x-user-id", "test-user".parse().unwrap());
    req
}
