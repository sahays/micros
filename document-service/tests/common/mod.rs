use document_service::config::DocumentConfig;
use document_service::services::MongoDb;
use document_service::startup::Application;
use service_core::config::Config as CoreConfig;
use service_core::grpc::{DocumentClient, DocumentClientConfig};
use std::time::Duration;
use uuid::Uuid;

// Test constants for tenant context
pub const TEST_APP_ID: &str = "test-app-id";
pub const TEST_ORG_ID: &str = "test-org-id";
pub const TEST_USER_ID: &str = "test_user_123";

pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub http_port: u16,
    pub grpc_port: u16,
    pub db: MongoDb,
    pub db_name: String,
    pub storage_path: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");

        let db_name = format!("document_test_{}", Uuid::new_v4());
        let storage_path = format!("target/test-storage-{}", Uuid::new_v4());

        let mut config = DocumentConfig::load().expect("Failed to load configuration");
        config.common.port = 0; // Random port for testing
        config.mongodb.database = db_name.clone();
        config.storage.local_path = storage_path.clone();

        let app = Application::build(config)
            .await
            .expect("Failed to build test application");

        let http_port = app.http_port();
        let grpc_port = app.grpc_port();
        let db = app.db().clone();
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
            db_name,
            storage_path,
        }
    }

    /// Create a gRPC client connected to this test app.
    pub async fn grpc_client(&self) -> DocumentClient {
        DocumentClient::new(DocumentClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(60),
        })
        .await
        .expect("Failed to connect to gRPC server")
    }

    /// Cleanup test resources (database and storage).
    pub async fn cleanup(&self) {
        let _ = self.db.client().database(&self.db_name).drop(None).await;
        let _ = tokio::fs::remove_dir_all(&self.storage_path).await;
    }
}
