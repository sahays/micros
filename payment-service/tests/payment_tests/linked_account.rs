use crate::common::embedded::{TestApp, TEST_APP_ID, TEST_TENANT_ID, TEST_USER_ID};
use serial_test::serial;
use service_core::grpc::proto::payment::{
    BankAccount, CommissionConfig, CommissionType, LegalInfo, LinkedAccountStatus,
};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

fn mock_razorpay_account_response() -> serde_json::Value {
    serde_json::json!({
        "id": "acc_test_123",
        "type": "route",
        "status": "created",
        "email": "vendor@example.com",
        "legal_business_name": "Vendor Inc",
        "contact_name": "Test Vendor"
    })
}

fn test_bank_account() -> BankAccount {
    BankAccount {
        account_holder_name: "Test Vendor".to_string(),
        account_number: "1234567890".to_string(),
        ifsc_code: "SBIN0001234".to_string(),
        account_type: "savings".to_string(),
    }
}

fn test_legal_info() -> LegalInfo {
    LegalInfo {
        legal_business_name: "Vendor Inc".to_string(),
        business_type: "individual".to_string(),
        pan: Some("ABCDE1234F".to_string()),
        gst: None,
    }
}

#[tokio::test]
#[serial]
async fn create_linked_account_via_grpc() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    let account = client
        .create_linked_account(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            test_bank_account(),
            test_legal_info(),
            None,
        )
        .await
        .expect("Failed to create linked account");

    assert!(!account.id.is_empty());
    assert_eq!(account.name, "Test Vendor");
    assert_eq!(account.email, "vendor@example.com");
    assert_eq!(account.razorpay_account_id, "acc_test_123");
    assert_eq!(account.status, LinkedAccountStatus::Created as i32);
}

#[tokio::test]
#[serial]
async fn get_linked_account_not_found() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    let result = client
        .get_linked_account(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "nonexistent-id",
        )
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
#[serial]
async fn get_linked_account_after_create() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .create_linked_account(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            test_bank_account(),
            test_legal_info(),
            None,
        )
        .await
        .expect("Failed to create linked account");

    let fetched = client
        .get_linked_account(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), &created.id)
        .await
        .expect("Failed to get linked account");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, "Test Vendor");
}

#[tokio::test]
#[serial]
async fn list_linked_accounts_filters_by_status() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    client
        .create_linked_account(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            test_bank_account(),
            test_legal_info(),
            None,
        )
        .await
        .expect("Failed to create linked account");

    // List all
    let (accounts, total) = client
        .list_linked_accounts(TEST_APP_ID, TEST_TENANT_ID, Some(TEST_USER_ID), None, 10, 0)
        .await
        .expect("Failed to list linked accounts");

    assert_eq!(accounts.len(), 1);
    assert_eq!(total, 1);

    // List by ACTIVATED status (none should match)
    let (accounts, total) = client
        .list_linked_accounts(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            Some(LinkedAccountStatus::Activated),
            10,
            0,
        )
        .await
        .expect("Failed to list linked accounts");

    assert_eq!(accounts.len(), 0);
    assert_eq!(total, 0);
}

#[tokio::test]
#[serial]
async fn update_commission_config() {
    let app = TestApp::shared();
    app.reset().await;
    let mut client = app.grpc_client().await;

    Mock::given(method("POST"))
        .and(path_regex("/v2/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_razorpay_account_response()))
        .mount(&app.razorpay_mock)
        .await;

    let created = client
        .create_linked_account(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            "Test Vendor",
            "vendor@example.com",
            test_bank_account(),
            test_legal_info(),
            None,
        )
        .await
        .expect("Failed to create linked account");

    let updated = client
        .update_commission_config(
            TEST_APP_ID,
            TEST_TENANT_ID,
            Some(TEST_USER_ID),
            &created.id,
            CommissionConfig {
                commission_type: CommissionType::Percentage as i32,
                value: 500, // 5% in basis points
            },
        )
        .await
        .expect("Failed to update commission config");

    assert_eq!(updated.id, created.id);
    let commission = updated.commission.expect("Commission should be set");
    assert_eq!(
        commission.commission_type,
        CommissionType::Percentage as i32
    );
    assert_eq!(commission.value, 500);
}
