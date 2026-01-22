use payment_service::config::{
    Config, DatabaseConfig, RazorpayConfig, RedisConfig, ServerConfig, ServiceSignatureConfig,
    UpiConfig,
};
use payment_service::startup::Application;
use secrecy::Secret;
use service_core::grpc::{PaymentClient, PaymentClientConfig};
use std::time::Duration;

pub const TEST_APP_ID: &str = "test-app";
pub const TEST_ORG_ID: &str = "test-org";
pub const TEST_USER_ID: &str = "test-user";

pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub http_port: u16,
    pub grpc_port: u16,
    pub db: mongodb::Database,
    pub db_name: String,
}

impl TestApp {
    pub async fn spawn() -> Self {
        let db_name = format!("payment_test_{}", uuid::Uuid::new_v4());

        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,      // Random port
                grpc_port: 0, // Will be http_port + 1
            },
            database: DatabaseConfig {
                url: Secret::new(
                    std::env::var("TEST_MONGODB_URI")
                        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
                ),
                db_name: db_name.clone(),
            },
            redis: RedisConfig {
                url: Secret::new(
                    std::env::var("TEST_REDIS_URL")
                        .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                ),
            },
            signature: ServiceSignatureConfig {
                enabled: false,
                secret: Secret::new("test-secret".to_string()),
                expiry_seconds: 300,
            },
            upi: UpiConfig {
                vpa: "test@upi".to_string(),
                merchant_name: "Test Merchant".to_string(),
            },
            razorpay: RazorpayConfig {
                key_id: "test_key_id".to_string(),
                key_secret: Secret::new("test_key_secret".to_string()),
                webhook_secret: Secret::new("test_webhook_secret".to_string()),
                api_base_url: "https://api.razorpay.com/v1".to_string(),
            },
            service_name: "payment-service-test".to_string(),
        };

        let app = Application::build(config)
            .await
            .expect("Failed to build test application");

        let http_port = app.http_port();
        let grpc_port = app.grpc_port();
        let http_address = format!("http://127.0.0.1:{}", http_port);
        let grpc_address = format!("http://127.0.0.1:{}", grpc_port);
        let db = app.db().clone();

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
        }
    }

    /// Create a gRPC client connected to this test app.
    pub async fn grpc_client(&self) -> PaymentClient {
        PaymentClient::new(PaymentClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        })
        .await
        .expect("Failed to connect to gRPC server")
    }

    /// Cleanup test database after test completes.
    pub async fn cleanup(&self) {
        self.db
            .drop(None)
            .await
            .expect("Failed to drop test database");
    }
}
