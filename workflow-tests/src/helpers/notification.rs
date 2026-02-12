#![allow(dead_code)]

use service_core::grpc::{NotificationClient, NotificationClientConfig};
use std::time::Duration;

fn grpc_endpoint() -> String {
    std::env::var("NOTIFICATION_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:50053".to_string())
}

fn http_base_url() -> String {
    std::env::var("NOTIFICATION_HTTP_URL").unwrap_or_else(|_| "http://localhost:9008".to_string())
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

    /// Create a gRPC client connected to the deployed notification-service.
    pub async fn grpc_client(&self) -> NotificationClient {
        NotificationClient::new(NotificationClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        })
        .await
        .expect("Failed to connect to notification gRPC server")
    }
}
