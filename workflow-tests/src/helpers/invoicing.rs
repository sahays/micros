#![allow(dead_code)]

use crate::proto::invoicing::invoicing_service_client::InvoicingServiceClient;
use tonic::transport::Channel;
use uuid::Uuid;

fn grpc_endpoint() -> String {
    std::env::var("INVOICING_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:50059".to_string())
}

fn http_base_url() -> String {
    std::env::var("INVOICING_HTTP_URL")
        .unwrap_or_else(|_| "http://localhost:9014".to_string())
}

/// Test application wrapper for integration tests.
pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub tenant_id: String,
    pub customer_id: String,
}

impl TestApp {
    /// Connect to the deployed invoicing-service.
    pub async fn spawn() -> Self {
        TestApp {
            http_address: http_base_url(),
            grpc_address: grpc_endpoint(),
            tenant_id: Uuid::new_v4().to_string(),
            customer_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a gRPC client connected to the deployed invoicing-service.
    pub async fn grpc_client(&self) -> InvoicingServiceClient<Channel> {
        InvoicingServiceClient::connect(self.grpc_address.clone())
            .await
            .expect("Failed to connect to invoicing gRPC server")
    }

    /// Get test tenant ID.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Get test customer ID.
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }
}

/// Helper to create a request with tenant metadata.
pub fn with_tenant<T>(tenant_id: &str, request: T) -> tonic::Request<T> {
    let mut req = tonic::Request::new(request);
    req.metadata_mut()
        .insert("x-tenant-id", tenant_id.parse().unwrap());
    req
}
