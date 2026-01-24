//! Integration tests for adjustment operations.

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

/// Helper to create a reconciliation and get back the ID.
async fn create_reconciliation(
    client: &mut reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient<tonic::transport::Channel>,
    tenant_id: &Uuid,
    bank_account_id: &str,
) -> String {
    let request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: bank_account_id.to_string(),
            period_start: "2024-01-01".to_string(),
            period_end: "2024-01-31".to_string(),
        },
        tenant_id,
    );

    client
        .start_reconciliation(request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap()
        .reconciliation_id
}

#[tokio::test]
async fn create_adjustment_success() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let reconciliation_id =
        create_reconciliation(&mut client, &app.tenant_id, &bank_account_id).await;

    let request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: reconciliation_id.clone(),
            adjustment_type: AdjustmentType::BankFee.into(),
            description: "Monthly service fee".to_string(),
            amount: "-15.00".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.create_adjustment(request).await;
    assert!(response.is_ok());

    let adjustment = response.unwrap().into_inner().adjustment.unwrap();
    assert_eq!(adjustment.reconciliation_id, reconciliation_id);
    assert_eq!(adjustment.description, "Monthly service fee");
    assert_eq!(adjustment.amount, "-15.00");
}

#[tokio::test]
async fn create_adjustment_with_timing_difference() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let reconciliation_id =
        create_reconciliation(&mut client, &app.tenant_id, &bank_account_id).await;

    let request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: reconciliation_id.clone(),
            adjustment_type: AdjustmentType::TimingDifference.into(),
            description: "Check not yet cleared".to_string(),
            amount: "500.00".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.create_adjustment(request).await;
    assert!(response.is_ok());

    let adjustment = response.unwrap().into_inner().adjustment.unwrap();
    assert_eq!(
        adjustment.adjustment_type,
        AdjustmentType::TimingDifference as i32
    );
}

#[tokio::test]
async fn create_adjustment_fails_for_nonexistent_reconciliation() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: Uuid::new_v4().to_string(),
            adjustment_type: AdjustmentType::BankFee.into(),
            description: "Test adjustment".to_string(),
            amount: "10.00".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.create_adjustment(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn create_adjustment_fails_with_empty_description() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let reconciliation_id =
        create_reconciliation(&mut client, &app.tenant_id, &bank_account_id).await;

    let request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: reconciliation_id.clone(),
            adjustment_type: AdjustmentType::BankFee.into(),
            description: "".to_string(),
            amount: "10.00".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.create_adjustment(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn create_adjustment_fails_for_completed_reconciliation() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let reconciliation_id =
        create_reconciliation(&mut client, &app.tenant_id, &bank_account_id).await;

    // Complete the reconciliation
    let complete_request = with_tenant(
        CompleteReconciliationRequest {
            reconciliation_id: reconciliation_id.clone(),
        },
        &app.tenant_id,
    );
    client
        .complete_reconciliation(complete_request)
        .await
        .unwrap();

    // Try to add adjustment - should fail
    let request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: reconciliation_id.clone(),
            adjustment_type: AdjustmentType::BankFee.into(),
            description: "Late fee".to_string(),
            amount: "25.00".to_string(),
        },
        &app.tenant_id,
    );

    let response = client.create_adjustment(request).await;
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );
}

#[tokio::test]
async fn list_adjustments_returns_adjustments() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let bank_account_id = create_bank_account(&mut client, &app.tenant_id).await;
    let reconciliation_id =
        create_reconciliation(&mut client, &app.tenant_id, &bank_account_id).await;

    // Create multiple adjustments
    for (desc, amount) in [
        ("Bank fee", "-15.00"),
        ("Interest earned", "5.00"),
        ("Outstanding check", "-100.00"),
    ] {
        let request = with_tenant(
            CreateAdjustmentRequest {
                reconciliation_id: reconciliation_id.clone(),
                adjustment_type: AdjustmentType::BankFee.into(),
                description: desc.to_string(),
                amount: amount.to_string(),
            },
            &app.tenant_id,
        );
        client.create_adjustment(request).await.unwrap();
    }

    // List adjustments
    let list_request = with_tenant(
        ListAdjustmentsRequest {
            reconciliation_id: reconciliation_id.clone(),
            page_size: 10,
            page_token: None,
        },
        &app.tenant_id,
    );

    let response = client.list_adjustments(list_request).await.unwrap();
    let adjustments = response.into_inner().adjustments;

    assert_eq!(adjustments.len(), 3);
}

#[tokio::test]
async fn tenant_isolation_for_adjustments() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create bank account and reconciliation for tenant1
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

    let recon_request = with_tenant(
        StartReconciliationRequest {
            bank_account_id: account1.bank_account_id.clone(),
            period_start: "2024-02-01".to_string(),
            period_end: "2024-02-28".to_string(),
        },
        &tenant1,
    );

    let reconciliation = client
        .start_reconciliation(recon_request)
        .await
        .unwrap()
        .into_inner()
        .reconciliation
        .unwrap();

    // Create adjustment for tenant1
    let adj_request = with_tenant(
        CreateAdjustmentRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
            adjustment_type: AdjustmentType::BankFee.into(),
            description: "Tenant1 adjustment".to_string(),
            amount: "10.00".to_string(),
        },
        &tenant1,
    );
    client.create_adjustment(adj_request).await.unwrap();

    // Tenant2 should not see tenant1's adjustments
    let list_request = with_tenant(
        ListAdjustmentsRequest {
            reconciliation_id: reconciliation.reconciliation_id.clone(),
            page_size: 10,
            page_token: None,
        },
        &tenant2,
    );

    let response = client.list_adjustments(list_request).await.unwrap();
    // Should return empty list (not fail) since reconciliation doesn't belong to tenant2
    assert!(response.into_inner().adjustments.is_empty());
}
