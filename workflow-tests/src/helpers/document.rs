#![allow(dead_code)]

use service_core::grpc::{DocumentClient, DocumentClientConfig};
use std::time::Duration;
use uuid::Uuid;

pub const TEST_APP_ID: &str = "test-app-id";
pub const TEST_ORG_ID: &str = "test-org-id";

fn grpc_endpoint() -> String {
    std::env::var("DOCUMENT_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:50052".to_string())
}

fn http_base_url() -> String {
    std::env::var("DOCUMENT_HTTP_URL").unwrap_or_else(|_| "http://localhost:9007".to_string())
}

pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        TestApp {
            http_address: http_base_url(),
            grpc_address: grpc_endpoint(),
        }
    }

    pub async fn grpc_client(&self) -> DocumentClient {
        DocumentClient::new(DocumentClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(60),
        })
        .await
        .expect("Failed to connect to document gRPC server")
    }

    /// Generate a unique user ID for test isolation.
    pub fn test_user_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
}
