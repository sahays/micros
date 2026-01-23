//! Tax rate integration tests for invoicing-service.

mod common;

use common::{with_tenant, TestApp, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    CreateTaxRateRequest, GetTaxRateRequest, ListTaxRatesRequest, TaxCalculation,
    UpdateTaxRateRequest,
};

#[tokio::test]
async fn create_tax_rate_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "GST 18%".to_string(),
            rate: "0.18".to_string(),
            calculation: TaxCalculation::Exclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: String::new(),
        },
    );

    let response = client
        .create_tax_rate(request)
        .await
        .expect("Failed to create tax rate");
    let tax_rate = response.into_inner().tax_rate.expect("Missing tax rate");

    assert!(!tax_rate.tax_rate_id.is_empty());
    assert_eq!(tax_rate.name, "GST 18%");
    assert_eq!(tax_rate.rate, "0.18");
    assert_eq!(tax_rate.calculation, TaxCalculation::Exclusive as i32);
    assert!(tax_rate.active);

    app.cleanup().await;
}

#[tokio::test]
async fn create_tax_rate_with_end_date_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Promotional Tax 10%".to_string(),
            rate: "0.10".to_string(),
            calculation: TaxCalculation::Inclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: "2026-06-30".to_string(),
        },
    );

    let response = client
        .create_tax_rate(request)
        .await
        .expect("Failed to create tax rate");
    let tax_rate = response.into_inner().tax_rate.expect("Missing tax rate");

    assert_eq!(tax_rate.effective_to, "2026-06-30");

    app.cleanup().await;
}

#[tokio::test]
async fn get_tax_rate_returns_created_rate() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a tax rate first
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "VAT 20%".to_string(),
            rate: "0.20".to_string(),
            calculation: TaxCalculation::Exclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: String::new(),
        },
    );

    let create_response = client
        .create_tax_rate(create_request)
        .await
        .expect("Failed to create tax rate");
    let created_rate = create_response
        .into_inner()
        .tax_rate
        .expect("Missing tax rate");

    // Get the tax rate
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            tax_rate_id: created_rate.tax_rate_id.clone(),
        },
    );

    let get_response = client
        .get_tax_rate(get_request)
        .await
        .expect("Failed to get tax rate");
    let tax_rate = get_response
        .into_inner()
        .tax_rate
        .expect("Missing tax rate");

    assert_eq!(tax_rate.tax_rate_id, created_rate.tax_rate_id);
    assert_eq!(tax_rate.name, "VAT 20%");

    app.cleanup().await;
}

#[tokio::test]
async fn get_tax_rate_not_found_returns_error() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            tax_rate_id: "99999999-9999-9999-9999-999999999999".to_string(),
        },
    );

    let result = client.get_tax_rate(get_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_tax_rates_returns_tenant_rates() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a few tax rates
    let rates = vec![
        ("Standard Rate", "0.18"),
        ("Reduced Rate", "0.05"),
        ("Zero Rate", "0.00"),
    ];

    for (name, rate) in rates {
        let create_request = with_tenant(
            TEST_TENANT_ID,
            CreateTaxRateRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                name: name.to_string(),
                rate: rate.to_string(),
                calculation: TaxCalculation::Exclusive as i32,
                effective_from: "2026-01-01".to_string(),
                effective_to: String::new(),
            },
        );

        client
            .create_tax_rate(create_request)
            .await
            .expect("Failed to create tax rate");
    }

    // List tax rates
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListTaxRatesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            active_only: false,
            as_of_date: String::new(),
            page_size: 10,
            page_token: String::new(),
        },
    );

    let list_response = client
        .list_tax_rates(list_request)
        .await
        .expect("Failed to list tax rates");
    let tax_rates = list_response.into_inner().tax_rates;

    assert!(tax_rates.len() >= 3);
    for tax_rate in &tax_rates {
        assert_eq!(tax_rate.tenant_id, TEST_TENANT_ID);
    }

    app.cleanup().await;
}

#[tokio::test]
async fn list_active_tax_rates_filters_inactive() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create an active tax rate
    let create_active = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Active Rate".to_string(),
            rate: "0.15".to_string(),
            calculation: TaxCalculation::Exclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: String::new(),
        },
    );

    let active_response = client
        .create_tax_rate(create_active)
        .await
        .expect("Failed to create tax rate");
    let active_rate = active_response
        .into_inner()
        .tax_rate
        .expect("Missing tax rate");

    // Deactivate it
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            tax_rate_id: active_rate.tax_rate_id.clone(),
            name: String::new(),
            rate: String::new(),
            calculation: TaxCalculation::Unspecified as i32,
            effective_from: String::new(),
            effective_to: String::new(),
            active: false,
        },
    );

    client
        .update_tax_rate(update_request)
        .await
        .expect("Failed to update tax rate");

    // Create another active rate
    let create_active2 = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Still Active Rate".to_string(),
            rate: "0.12".to_string(),
            calculation: TaxCalculation::Exclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: String::new(),
        },
    );

    client
        .create_tax_rate(create_active2)
        .await
        .expect("Failed to create tax rate");

    // List only active tax rates
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListTaxRatesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            active_only: true,
            as_of_date: "2026-01-15".to_string(),
            page_size: 10,
            page_token: String::new(),
        },
    );

    let list_response = client
        .list_tax_rates(list_request)
        .await
        .expect("Failed to list tax rates");
    let tax_rates = list_response.into_inner().tax_rates;

    // All returned rates should be active
    for tax_rate in &tax_rates {
        assert!(tax_rate.active, "Expected only active tax rates");
    }

    // The deactivated rate should not be in the list
    let deactivated_found = tax_rates
        .iter()
        .any(|r| r.tax_rate_id == active_rate.tax_rate_id);
    assert!(
        !deactivated_found,
        "Deactivated rate should not be in active list"
    );

    app.cleanup().await;
}

#[tokio::test]
async fn update_tax_rate_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a tax rate
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            name: "Update Test Rate".to_string(),
            rate: "0.10".to_string(),
            calculation: TaxCalculation::Exclusive as i32,
            effective_from: "2026-01-01".to_string(),
            effective_to: String::new(),
        },
    );

    let create_response = client
        .create_tax_rate(create_request)
        .await
        .expect("Failed to create tax rate");
    let created_rate = create_response
        .into_inner()
        .tax_rate
        .expect("Missing tax rate");

    // Update the tax rate
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateTaxRateRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            tax_rate_id: created_rate.tax_rate_id.clone(),
            name: "Updated Rate Name".to_string(),
            rate: "0.12".to_string(),
            calculation: TaxCalculation::Unspecified as i32, // Don't change
            effective_from: String::new(),
            effective_to: "2026-12-31".to_string(),
            active: true,
        },
    );

    let update_response = client
        .update_tax_rate(update_request)
        .await
        .expect("Failed to update tax rate");
    let updated_rate = update_response
        .into_inner()
        .tax_rate
        .expect("Missing tax rate");

    assert_eq!(updated_rate.name, "Updated Rate Name");
    assert_eq!(updated_rate.rate, "0.12");
    assert_eq!(updated_rate.effective_to, "2026-12-31");

    app.cleanup().await;
}
