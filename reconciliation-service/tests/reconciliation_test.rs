//! Integration tests for reconciliation process operations.

mod common;

use common::{spawn_app, with_tenant};
use reconciliation_service::grpc::proto::*;
use uuid::Uuid;

/// Helper to create a bank account and get back the ID.
async fn create_bank_account(
    client: &mut reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient<tonic::transport::Channel>,
    tenant_id: &Uuid,
) -> String {
    let request = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Test Bank".to_string(),
            account_number_masked: "****1234".to_string(),
            currency: "USD".to_string(),
        },
        tenant_id,
    );

    client
        .register_bank_account(request)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap()
        .bank_account_id
}

#[tokio::test]
async fn start_reconciliation_creates_in_progress_reconciliation() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    let request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.clone(),
            period_start: "2024-01-01".to_string(),
            period_end: "2024-01-31".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.start_reconciliation(request).await;
    assert!(response.is_ok());

    let reconciliation = response.unwrap().into_inner().reconciliation.unwrap();
    assert_eq!(reconciliation.bank_account_id, bank_account_id);
    assert_eq!(reconciliation.period_start, "2024-01-01");
    assert_eq!(reconciliation.period_end, "2024-01-31");
    assert_eq!(
        reconciliation.status,
        ReconciliationStatus::InProgress as i32
    );
}

#[tokio::test]
async fn start_reconciliation_fails_for_nonexistent_bank_account() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: Uuid::new_v4().to_string(),
            period_start: "2024-01-01".to_string(),
            period_end: "2024-01-31".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.start_reconciliation(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn get_reconciliation_returns_reconciliation() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    // Start a reconciliation
    let start_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.clone(),
            period_start: "2024-02-01".to_string(),
            period_end: "2024-02-28".to_string(),
        },
        &app.tenant_id,
    );

    let reconciliation = client
        .start_reconciliation(start_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    // Get the reconciliation
    let get_request = with_tenant(
        GetReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.get_reconciliation(get_request).await;
    assert!(response.is_ok());

    let fetched = response.unwrap().into_inner().reconciliation.unwrap();
    assert_eq!(fetched.reconciliation_id, reconciliation.reconciliation_id);
    assert_eq!(fetched.bank_account_id, bank_account_id);
}

#[tokio::test]
async fn list_reconciliations_returns_reconciliations() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    // Create multiple reconciliations
    for month in 1..=3 {
        let start_request = with_tenant(
            StartReconciliationRequest {
                bank_account_id: bank_account_id.clone(),
                period_start: format!("2024-{:02}-01", month),
                period_end: format!("2024-{:02}-28", month),
            },
            &app.tenant_id,
        );
        client.start_reconciliation(start_request).await.unwrap();
    }

    // List reconciliations
    let list_request = with_tenant(
        ListReconciliationsRequest {
            bank_account_id: bank_account_id.clone(),
            page_size: 10,
            page_token: None,
            status_filter: None,
        },
        &app.tenant_id,
    );

    let response = client.list_reconciliations(list_request).await.unwrap();
    let reconciliations = response.into_inner().reconciliations;

    assert_eq!(reconciliations.len(), 3);
}

#[tokio::test]
async fn complete_reconciliation_changes_status() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    // Start a reconciliation
    let start_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.clone(),
            period_start: "2024-03-01".to_string(),
            period_end: "2024-03-31".to_string(),
        },
        &app.tenant_id,
    );

    let reconciliation = client
        .start_reconciliation(start_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    // Complete the reconciliation
    let complete_request = with_tenant(
        CompleteReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.complete_reconciliation(complete_request).await;
    assert!(response.is_ok());

    let completed = response.unwrap().into_inner().reconciliation.unwrap();
    assert_eq!(completed.status, ReconciliationStatus::Completed as i32);

    // Verify bank account is updated
    let account_request = with_tenant(
        GetBankAccountRequest {
            bank_account_id: bank_account_id.clone(),
        },
        &app.tenant_id,
    );

    let account = client
        .get_bank_account(account_request)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Last reconciled date should be updated
    assert!(account.last_reconciled_date.is_some());
}

#[tokio::test]
async fn complete_reconciliation_fails_if_not_in_progress() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    // Start and complete a reconciliation
    let start_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.clone(),
            period_start: "2024-04-01".to_string(),
            period_end: "2024-04-30".to_string(),
        },
        &app.tenant_id,
    );

    let reconciliation = client
        .start_reconciliation(start_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    let complete_request = with_tenant(
        CompleteReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    client
        .complete_reconciliation(complete_request)
        .await
        .unwrap();

    // Try to complete again - should fail
    let complete_again_request = with_tenant(
        CompleteReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.complete_reconciliation(complete_again_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::Internal);
}

#[tokio::test]
async fn abandon_reconciliation_changes_status() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;

    // Start a reconciliation
    let start_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.clone(),
            period_start: "2024-05-01".to_string(),
            period_end: "2024-05-31".to_string(),
        },
        &app.tenant_id,
    );

    let reconciliation = client
        .start_reconciliation(start_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    // Abandon the reconciliation
    let abandon_request = with_tenant(
        AbandonReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.abandon_reconciliation(abandon_request).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().success);

    // Verify status is abandoned
    let get_request = with_tenant(
        GetReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &app.tenant_id,
    );

    let fetched = client
        .get_reconciliation(get_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    assert_eq!(fetched.status, ReconciliationStatus::Abandoned as i32);
}

#[tokio::test]
async fn tenant_isolation_for_reconciliations() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create bank account for tenant1
    let request1 = with_tenant(
        RegisterBankAccountRequest {
            ledger_account_id: Uuid::new_v4().to_string(),
            bank_name: "Tenant1 Bank".to_string(),
            account_number_masked: "****1111".to_string(),
            currency: "USD".to_string(),
        },
        &tenant1,
    );

    let account1 = client
        .register_bank_account(request1)
        .await
        .unwrap()
        .into_inner()
        .bank_account
        .unwrap();

    // Start reconciliation for tenant1
    let start_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: account1.bank_account_id.clone(),
            period_start: "2024-06-01".to_string(),
            period_end: "2024-06-30".to_string(),
        },
        &tenant1,
    );

    let reconciliation = client
        .start_reconciliation(start_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    // Tenant2 should not be able to access tenant1's reconciliation
    let get_request = with_tenant(
        GetReconciliationRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
        },
        &tenant2,
    );

    let response = client.get_reconciliation(get_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}
