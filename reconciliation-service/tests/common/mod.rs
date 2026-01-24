//! Common test utilities for reconciliation-service integration tests.

use reconciliation_service::config::{
    AuthConfig, DatabaseConfig, DocumentServiceConfig, GenaiServiceConfig, LedgerServiceConfig,
    ReconciliationConfig,
};
use reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient;
use reconciliation_service::startup::Application;
use service_core::config::Config as CommonConfig;
use std::sync::Once;
use tonic::transport::Channel;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize tracing for tests (only once).
pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,reconciliation_service=debug,sqlx=warn")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

/// Test configuration with empty auth endpoint (disables capability checking).
fn test_config() -> ReconciliationConfig {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set - use scripts/integ-tests.sh to run tests");

    ReconciliationConfig {
        common: CommonConfig { port: 0 },
        service_name: "reconciliation-service-test".to_string(),
        service_version: "test".to_string(),
        log_level: "debug".to_string(),
        otlp_endpoint: None,
        database: DatabaseConfig {
            url: database_url,
            max_connections: 2,
            min_connections: 1,
        },
        ledger_service: LedgerServiceConfig {
            url: String::new(), // Empty = skip ledger validation in tests
        },
        genai_service: GenaiServiceConfig { url: String::new() },
        document_service: DocumentServiceConfig { url: String::new() },
        auth: AuthConfig {
            auth_service_endpoint: String::new(), // Empty = disable capability checking
        },
    }
}

/// Test application wrapper.
#[allow(dead_code)]
pub struct TestApp {
    pub grpc_client: ReconciliationServiceClient<Channel>,
    pub tenant_id: Uuid,
    pub http_port: u16,
    pub grpc_port: u16,
}

/// Spawn a test application and return the gRPC client with a unique tenant ID.
pub async fn spawn_app() -> TestApp {
    init_tracing();

    let config = test_config();

    // Use build_without_migrations since integ-tests.sh already ran migrations
    let app = Application::build_without_migrations(config)
        .await
        .expect("Failed to build application");

    let http_port = app.http_port();
    let grpc_port = app.grpc_port();
    let grpc_addr = format!("http://127.0.0.1:{}", grpc_port);

    // Start the application in the background
    tokio::spawn(async move {
        app.run_until_stopped().await.ok();
    });

    // Wait for server to be ready with retry
    let grpc_client = {
        let mut attempts = 0;
        loop {
            match ReconciliationServiceClient::connect(grpc_addr.clone()).await {
                Ok(client) => break client,
                Err(_) if attempts < 20 => {
                    attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                Err(e) => panic!("Failed to connect gRPC client after 20 attempts: {}", e),
            }
        }
    };

    let tenant_id = Uuid::new_v4();
    TestApp {
        grpc_client,
        tenant_id,
        http_port,
        grpc_port,
    }
}

/// Helper to inject tenant ID and user ID into request metadata.
/// This is needed because capability checking is disabled, so we need to inject
/// the auth context directly.
pub fn with_tenant<T>(request: T, tenant_id: &Uuid) -> tonic::Request<T> {
    let mut req = tonic::Request::new(request);
    // Inject tenant ID and user ID as metadata for when capability checking is disabled
    req.metadata_mut()
        .insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    req.metadata_mut()
        .insert("x-user-id", "test-user".parse().unwrap());
    req
}
