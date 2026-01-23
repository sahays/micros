//! Invoice CRUD integration tests for invoicing-service.

mod common;

use common::{with_tenant, TestApp, TEST_CUSTOMER_ID, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    Address, CreateInvoiceRequest, DeleteInvoiceRequest, GetInvoiceRequest, InvoiceStatus,
    InvoiceType, ListInvoicesRequest, UpdateInvoiceRequest,
};

/// Helper to create a billing address for tests.
fn test_address(line1: &str, city: &str, state: &str) -> Option<Address> {
    Some(Address {
        line1: line1.to_string(),
        line2: String::new(),
        city: city.to_string(),
        state: state.to_string(),
        postal_code: "12345".to_string(),
        country: "US".to_string(),
    })
}

#[tokio::test]
async fn create_invoice_returns_draft_invoice() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Test Customer".to_string(),
            billing_address: test_address("123 Test Street", "Test City", "TS"),
            currency: "USD".to_string(),
            due_date: "2026-02-28".to_string(),
            notes: "Test invoice".to_string(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let response = client
        .create_invoice(request)
        .await
        .expect("Failed to create invoice");
    let invoice = response.into_inner().invoice.expect("Missing invoice");

    assert!(!invoice.invoice_id.is_empty());
    assert_eq!(invoice.tenant_id, TEST_TENANT_ID);
    assert_eq!(invoice.customer_name, "Test Customer");
    assert_eq!(invoice.status, InvoiceStatus::Draft as i32);
    assert!(invoice.invoice_number.is_empty()); // Draft has no number

    app.cleanup().await;
}

#[tokio::test]
async fn get_invoice_returns_created_invoice() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create an invoice first
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Get Test Customer".to_string(),
            billing_address: test_address("456 Get Street", "Get City", "GT"),
            currency: "USD".to_string(),
            due_date: "2026-03-15".to_string(),
            notes: String::new(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let create_response = client
        .create_invoice(create_request)
        .await
        .expect("Failed to create invoice");
    let created_invoice = create_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    // Get the invoice
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: created_invoice.invoice_id.clone(),
        },
    );

    let get_response = client
        .get_invoice(get_request)
        .await
        .expect("Failed to get invoice");
    let invoice = get_response.into_inner().invoice.expect("Missing invoice");

    assert_eq!(invoice.invoice_id, created_invoice.invoice_id);
    assert_eq!(invoice.customer_name, "Get Test Customer");

    app.cleanup().await;
}

#[tokio::test]
async fn get_invoice_not_found_returns_error() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: "99999999-9999-9999-9999-999999999999".to_string(),
        },
    );

    let result = client.get_invoice(get_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn update_draft_invoice_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create an invoice first
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Original Name".to_string(),
            billing_address: test_address("Original Address", "Original City", "OG"),
            currency: "USD".to_string(),
            due_date: "2026-02-28".to_string(),
            notes: String::new(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let create_response = client
        .create_invoice(create_request)
        .await
        .expect("Failed to create invoice");
    let created_invoice = create_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    // Update the invoice
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: created_invoice.invoice_id.clone(),
            customer_name: "Updated Name".to_string(),
            billing_address: Some(Address {
                line1: "Updated Address".to_string(),
                line2: "Suite 100".to_string(),
                city: "Updated City".to_string(),
                state: "UP".to_string(),
                postal_code: "22222".to_string(),
                country: "US".to_string(),
            }),
            due_date: "2026-03-31".to_string(),
            notes: "Updated notes".to_string(),
            metadata: r#"{"updated": true}"#.to_string(),
        },
    );

    let update_response = client
        .update_invoice(update_request)
        .await
        .expect("Failed to update invoice");
    let updated_invoice = update_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    assert_eq!(updated_invoice.customer_name, "Updated Name");
    let addr = updated_invoice
        .billing_address
        .as_ref()
        .expect("Missing billing address");
    assert_eq!(addr.line1, "Updated Address");
    assert_eq!(updated_invoice.notes, "Updated notes");

    app.cleanup().await;
}

#[tokio::test]
async fn delete_draft_invoice_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create an invoice first
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Delete Test Customer".to_string(),
            billing_address: test_address("Delete Street", "Delete City", "DL"),
            currency: "USD".to_string(),
            due_date: "2026-02-28".to_string(),
            notes: String::new(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let create_response = client
        .create_invoice(create_request)
        .await
        .expect("Failed to create invoice");
    let created_invoice = create_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    // Delete the invoice
    let delete_request = with_tenant(
        TEST_TENANT_ID,
        DeleteInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: created_invoice.invoice_id.clone(),
        },
    );

    let delete_response = client
        .delete_invoice(delete_request)
        .await
        .expect("Failed to delete invoice");
    assert!(delete_response.into_inner().success);

    // Verify it's deleted
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: created_invoice.invoice_id,
        },
    );

    let result = client.get_invoice(get_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn list_invoices_returns_tenant_invoices() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create a few invoices
    for i in 0..3 {
        let create_request = with_tenant(
            TEST_TENANT_ID,
            CreateInvoiceRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                invoice_type: InvoiceType::Standard as i32,
                customer_id: TEST_CUSTOMER_ID.to_string(),
                customer_name: format!("List Test Customer {}", i),
                billing_address: test_address(&format!("{} List Street", i), "List City", "LS"),
                currency: "USD".to_string(),
                due_date: "2026-02-28".to_string(),
                notes: String::new(),
                reference_invoice_id: String::new(),
                metadata: "{}".to_string(),
            },
        );

        client
            .create_invoice(create_request)
            .await
            .expect("Failed to create invoice");
    }

    // List invoices
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListInvoicesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            status: InvoiceStatus::Draft as i32,
            customer_id: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            page_size: 10,
            page_token: String::new(),
        },
    );

    let list_response = client
        .list_invoices(list_request)
        .await
        .expect("Failed to list invoices");
    let invoices = list_response.into_inner().invoices;

    assert!(invoices.len() >= 3);
    for invoice in &invoices {
        assert_eq!(invoice.tenant_id, TEST_TENANT_ID);
    }

    app.cleanup().await;
}

#[tokio::test]
async fn list_invoices_pagination_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create 5 invoices
    for i in 0..5 {
        let create_request = with_tenant(
            TEST_TENANT_ID,
            CreateInvoiceRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                invoice_type: InvoiceType::Standard as i32,
                customer_id: TEST_CUSTOMER_ID.to_string(),
                customer_name: format!("Pagination Test {}", i),
                billing_address: test_address(&format!("{} Page Street", i), "Page City", "PG"),
                currency: "USD".to_string(),
                due_date: "2026-02-28".to_string(),
                notes: String::new(),
                reference_invoice_id: String::new(),
                metadata: "{}".to_string(),
            },
        );

        client
            .create_invoice(create_request)
            .await
            .expect("Failed to create invoice");
    }

    // First page
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListInvoicesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            status: InvoiceStatus::Draft as i32,
            customer_id: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            page_size: 2,
            page_token: String::new(),
        },
    );

    let first_page = client
        .list_invoices(list_request)
        .await
        .expect("Failed to list invoices");
    let first_response = first_page.into_inner();

    assert_eq!(first_response.invoices.len(), 2);
    assert!(!first_response.next_page_token.is_empty());

    // Second page
    let list_request2 = with_tenant(
        TEST_TENANT_ID,
        ListInvoicesRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            status: InvoiceStatus::Draft as i32,
            customer_id: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            page_size: 2,
            page_token: first_response.next_page_token,
        },
    );

    let second_page = client
        .list_invoices(list_request2)
        .await
        .expect("Failed to list invoices page 2");
    let second_response = second_page.into_inner();

    assert_eq!(second_response.invoices.len(), 2);

    app.cleanup().await;
}
