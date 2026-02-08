//! Embedded TestApp for payment-service tests that require wiremock (Razorpay mocking)
//! or direct database access.
//!
//! Uses a shared singleton TestApp per test binary to avoid the cost of spawning
//! a new server + MongoDB connection + index creation for every test.
//! The server runs on a dedicated background thread with its own Tokio runtime
//! so it outlives individual test runtimes.
//!
//! Used by: customer_test, refund_test, settlement_test, transfer_test,
//!          subscription_test, payment_link_test, linked_account_test

use payment_service::config::{
    AuthConfig, Config, DatabaseConfig, FeatureFlagsConfig, RazorpayConfig, RedisConfig,
    ServerConfig, ServiceSignatureConfig, UpiConfig,
};
use payment_service::startup::Application;
use secrecy::Secret;
use service_core::grpc::{PaymentClient, PaymentClientConfig};
use std::sync::OnceLock;
use std::time::Duration;
use wiremock::MockServer;

#[allow(dead_code)]
pub const TEST_APP_ID: &str = "test-app";
#[allow(dead_code)]
pub const TEST_ORG_ID: &str = "test-org";
#[allow(dead_code)]
pub const TEST_USER_ID: &str = "test-user";

static SHARED_APP: OnceLock<TestApp> = OnceLock::new();

const COLLECTION_NAMES: &[&str] = &[
    "transactions",
    "payment_methods",
    "linked_accounts",
    "customers",
    "transfers",
    "settlements",
    "plans",
    "subscriptions",
    "payment_links",
    "refunds",
];

#[allow(dead_code)]
pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub http_port: u16,
    pub grpc_port: u16,
    pub db: mongodb::Database,
    pub db_name: String,
    pub razorpay_mock: MockServer,
}

// Safety: TestApp fields are all Send+Sync. The MockServer communicates via channels.
unsafe impl Sync for TestApp {}

impl TestApp {
    /// Get or initialize the shared TestApp singleton for this test binary.
    /// The server runs on a dedicated background thread with its own Tokio runtime.
    /// Call `reset()` at the start of each test to ensure clean state.
    #[allow(dead_code)]
    pub fn shared() -> &'static TestApp {
        SHARED_APP.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build test runtime");

                rt.block_on(async {
                    let app = Self::spawn().await;
                    tx.send(app).expect("Failed to send TestApp");

                    // Keep the runtime alive so the server and wiremock continue serving
                    std::future::pending::<()>().await;
                });
            });

            rx.recv()
                .expect("Failed to receive TestApp from background thread")
        })
    }

    /// Reset wiremock expectations between tests.
    #[allow(dead_code)]
    pub async fn reset_mocks(&self) {
        self.razorpay_mock.reset().await;
    }

    /// Full reset: clear wiremock + truncate all collections.
    /// Use for tests that depend on empty collections (e.g., list_empty, duplicate checks).
    #[allow(dead_code)]
    pub async fn reset(&self) {
        self.razorpay_mock.reset().await;
        for name in COLLECTION_NAMES {
            self.db
                .collection::<mongodb::bson::Document>(name)
                .delete_many(mongodb::bson::doc! {}, None)
                .await
                .ok();
        }
    }

    async fn spawn() -> Self {
        let db_name = format!("payment_test_{}", uuid::Uuid::new_v4());
        let razorpay_mock = MockServer::start().await;

        let config = Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                grpc_port: 0,
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
                api_base_url: format!("{}/v1", razorpay_mock.uri()),
            },
            auth: AuthConfig {
                auth_service_endpoint: None,
            },
            feature_flags: FeatureFlagsConfig {
                razorpay_route_enabled: true,
                razorpay_subscriptions_enabled: true,
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
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Additional wait for gRPC server
        tokio::time::sleep(Duration::from_millis(100)).await;

        TestApp {
            http_address,
            grpc_address,
            http_port,
            grpc_port,
            db,
            db_name,
            razorpay_mock,
        }
    }

    #[allow(dead_code)]
    pub async fn grpc_client(&self) -> PaymentClient {
        PaymentClient::new(PaymentClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        })
        .await
        .expect("Failed to connect to gRPC server")
    }
}
