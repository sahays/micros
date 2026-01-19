use notification_service::config::{
    FcmConfig, MongoConfig, Msg91Config, NotificationConfig, SmtpConfig,
};
use notification_service::startup::Application;
use service_core::config::Config as CoreConfig;

pub struct TestApp {
    pub address: String,
    pub port: u16,
}

impl TestApp {
    pub async fn spawn() -> Self {
        // Use random port for testing
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
        };

        let app = Application::build(config)
            .await
            .expect("Failed to build test application");

        let port = app.port();
        let address = format!("http://127.0.0.1:{}", port);

        tokio::spawn(async move {
            app.run_until_stopped().await.ok();
        });

        // Wait for the server to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        TestApp { address, port }
    }
}
