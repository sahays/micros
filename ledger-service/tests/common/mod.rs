//! Common test utilities for ledger-service integration tests.

use ledger_service::config::{DatabaseConfig, LedgerConfig};
use ledger_service::grpc::proto::{
    ledger_service_client::LedgerServiceClient, AccountType as ProtoAccountType,
    CreateAccountRequest, CreateAccountResponse, Direction as ProtoDirection, GetBalanceRequest,
    GetBalanceResponse, PostTransactionEntry, PostTransactionRequest, PostTransactionResponse,
};
use ledger_service::startup::Application;
use service_core::config::Config as CommonConfig;
use std::sync::Once;
use tonic::transport::Channel;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize tracing for tests (only once).
pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,ledger_service=debug,sqlx=warn")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

/// Spawn a test application and return the gRPC client with a unique tenant ID.
pub async fn spawn_app() -> (LedgerServiceClient<Channel>, Uuid) {
    init_tracing();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set - use scripts/integ-tests.sh to run tests");

    let config = LedgerConfig {
        common: CommonConfig { port: 0 },
        service_name: "ledger-service-test".to_string(),
        service_version: "test".to_string(),
        log_level: "debug".to_string(),
        otlp_endpoint: None,
        database: DatabaseConfig {
            url: database_url,
            max_connections: 2,
            min_connections: 1,
        },
    };

    let app = Application::build(config)
        .await
        .expect("Failed to build application");

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
            match LedgerServiceClient::connect(grpc_addr.clone()).await {
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
    (grpc_client, tenant_id)
}

/// Helper to create an account for testing.
pub async fn create_test_account(
    client: &mut LedgerServiceClient<Channel>,
    tenant_id: Uuid,
    account_type: ProtoAccountType,
    account_code: &str,
    currency: &str,
    allow_negative: bool,
) -> CreateAccountResponse {
    let request = CreateAccountRequest {
        tenant_id: tenant_id.to_string(),
        account_type: account_type as i32,
        account_code: account_code.to_string(),
        currency: currency.to_string(),
        allow_negative,
        metadata: String::new(),
    };

    client
        .create_account(request)
        .await
        .expect("Failed to create account")
        .into_inner()
}

/// Helper to post a simple two-entry transaction.
pub async fn post_test_transaction(
    client: &mut LedgerServiceClient<Channel>,
    tenant_id: Uuid,
    debit_account_id: &str,
    credit_account_id: &str,
    amount: &str,
    effective_date: Option<&str>,
    idempotency_key: Option<&str>,
) -> PostTransactionResponse {
    let request = PostTransactionRequest {
        tenant_id: tenant_id.to_string(),
        entries: vec![
            PostTransactionEntry {
                account_id: debit_account_id.to_string(),
                amount: amount.to_string(),
                direction: ProtoDirection::Debit as i32,
            },
            PostTransactionEntry {
                account_id: credit_account_id.to_string(),
                amount: amount.to_string(),
                direction: ProtoDirection::Credit as i32,
            },
        ],
        effective_date: effective_date.unwrap_or("").to_string(),
        idempotency_key: idempotency_key.unwrap_or("").to_string(),
        metadata: String::new(),
    };

    client
        .post_transaction(request)
        .await
        .expect("Failed to post transaction")
        .into_inner()
}

/// Helper to get balance for an account.
pub async fn get_balance(
    client: &mut LedgerServiceClient<Channel>,
    tenant_id: Uuid,
    account_id: &str,
    as_of_date: Option<&str>,
) -> GetBalanceResponse {
    let request = GetBalanceRequest {
        tenant_id: tenant_id.to_string(),
        account_id: account_id.to_string(),
        as_of_date: as_of_date.unwrap_or("").to_string(),
    };

    client
        .get_balance(request)
        .await
        .expect("Failed to get balance")
        .into_inner()
}
