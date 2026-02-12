#![allow(dead_code)]

use crate::proto::ledger::{
    ledger_service_client::LedgerServiceClient, AccountType as ProtoAccountType,
    CreateAccountRequest, CreateAccountResponse, Direction as ProtoDirection, GetBalanceRequest,
    GetBalanceResponse, PostTransactionEntry, PostTransactionRequest, PostTransactionResponse,
};
use std::sync::Once;
use tonic::transport::Channel;
use tonic::Request;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize tracing for tests (only once).
pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,ledger_service=debug")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

fn grpc_endpoint() -> String {
    std::env::var("LEDGER_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:50056".to_string())
}

/// Connect to the deployed ledger-service and return the gRPC client with a unique tenant ID.
pub async fn spawn_app() -> (LedgerServiceClient<Channel>, Uuid) {
    init_tracing();

    let endpoint = grpc_endpoint();
    let grpc_client = LedgerServiceClient::connect(endpoint.clone())
        .await
        .unwrap_or_else(|e| panic!("Failed to connect to ledger-service at {}: {}", endpoint, e));

    let tenant_id = Uuid::new_v4();
    (grpc_client, tenant_id)
}

/// Wrap a gRPC request with BFF trust headers (x-tenant-id, x-user-id).
pub fn with_tenant<T>(inner: T, tenant_id: Uuid) -> Request<T> {
    let mut request = Request::new(inner);
    request
        .metadata_mut()
        .insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    request
        .metadata_mut()
        .insert("x-user-id", Uuid::new_v4().to_string().parse().unwrap());
    request
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
    let inner = CreateAccountRequest {
        tenant_id: tenant_id.to_string(),
        account_type: account_type as i32,
        account_code: account_code.to_string(),
        currency: currency.to_string(),
        allow_negative,
        metadata: String::new(),
    };

    client
        .create_account(with_tenant(inner, tenant_id))
        .await
        .expect("Failed to create account")
        .into_inner()
}

pub async fn post_test_transaction(
    client: &mut LedgerServiceClient<Channel>,
    tenant_id: Uuid,
    debit_account_id: &str,
    credit_account_id: &str,
    amount: &str,
    effective_date: Option<&str>,
    idempotency_key: Option<&str>,
) -> PostTransactionResponse {
    let inner = PostTransactionRequest {
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
        .post_transaction(with_tenant(inner, tenant_id))
        .await
        .expect("Failed to post transaction")
        .into_inner()
}

pub async fn get_balance(
    client: &mut LedgerServiceClient<Channel>,
    tenant_id: Uuid,
    account_id: &str,
    as_of_date: Option<&str>,
) -> GetBalanceResponse {
    let inner = GetBalanceRequest {
        tenant_id: tenant_id.to_string(),
        account_id: account_id.to_string(),
        as_of_date: as_of_date.unwrap_or("").to_string(),
    };

    client
        .get_balance(with_tenant(inner, tenant_id))
        .await
        .expect("Failed to get balance")
        .into_inner()
}
