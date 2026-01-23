use notification_service::config::{
    AuthConfig, FcmConfig, MongoConfig, Msg91Config, NotificationConfig, SmtpConfig,
};
use notification_service::startup::Application;
use service_core::config::Config as CoreConfig;
use service_core::grpc::{NotificationClient, NotificationClientConfig};
use std::time::Duration;

pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub http_port: u16,
    pub grpc_port: u16,
}

impl TestApp {
    pub async fn spawn() -> Self {
        // Use random port for testing (port 0)
        let config = NotificationConfig {
            common: CoreConfig { port: 0 },
            mongodb: MongoConfig {
                uri: std::env::var("TEST_MONGODB_URI")
                    .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
                database: format!("notification_test_{}", uuid::Uuid::new_v4()),
            },
            smtp: SmtpConfig {
                host: "smtp.test.local".to_string(),
                port: 587,
                user: "test".to_string(),
                password: "test".to_string(),
                from_email: "test@example.com".to_string(),
                from_name: "Test Service".to_string(),
                enabled: false, // Use mock
            },
            msg91: Msg91Config {
                auth_key: "test-key".to_string(),
                sender_id: "TEST".to_string(),
                enabled: false, // Use mock
            },
            fcm: FcmConfig {
                project_id: "test-project".to_string(),
                service_account_key: "test-key".to_string(),
                enabled: false, // Use mock
            },
            auth: AuthConfig {
                auth_service_endpoint: None, // Tests use BFF trust model
            },
        };

        let app = Application::build(config)
            .await
            .expect("Failed to build test application");

        let http_port = app.http_port();
        let grpc_port = app.grpc_port();
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
        }
    }

    /// Create a gRPC client connected to this test app.
    pub async fn grpc_client(&self) -> NotificationClient {
        NotificationClient::new(NotificationClientConfig {
            endpoint: self.grpc_address.clone(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        })
        .await
        .expect("Failed to connect to gRPC server")
    }
}
