#![allow(dead_code)]

use service_core::grpc::{PaymentClient, PaymentClientConfig};
use std::time::Duration;
use uuid::Uuid;

pub const TEST_APP_ID: &str = "test-app";
pub const TEST_ORG_ID: &str = "test-org";
pub const TEST_USER_ID: &str = "test-user";

fn grpc_endpoint() -> String {
    std::env::var("PAYMENT_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:50054".to_string())
}

fn http_base_url() -> String {
    std::env::var("PAYMENT_HTTP_URL").unwrap_or_else(|_| "http://localhost:9009".to_string())
}

/// Thin client TestApp that connects to a deployed payment-service.
pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        TestApp {
            http_address: http_base_url(),
            grpc_address: grpc_endpoint(),
            app_id: format!("test-app-{}", &Uuid::new_v4().to_string()[..8]),
            org_id: format!("test-org-{}", &Uuid::new_v4().to_string()[..8]),
            user_id: format!("test-user-{}", &Uuid::new_v4().to_string()[..8]),
        }
    }

    pub async fn grpc_client(&self) -> PaymentClient {
        PaymentClient::new(PaymentClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        })
        .await
        .expect("Failed to connect to payment gRPC server")
    }
}
