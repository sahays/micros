//! Invoice lifecycle integration tests for invoicing-service.
//! Tests for IssueInvoice and VoidInvoice operations.

mod common;

use common::{with_tenant, TestApp, TEST_CUSTOMER_ID, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    AddLineItemRequest, Address, CreateInvoiceRequest, InvoiceStatus, InvoiceType,
    IssueInvoiceRequest, UpdateInvoiceRequest, VoidInvoiceRequest,
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

/// Helper to create a draft invoice for testing.
async fn create_draft_invoice(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
    customer_name: &str,
) -> String {
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: customer_name.to_string(),
            billing_address: test_address("123 Test Street", "Test City", "TS"),
            currency: "USD".to_string(),
            due_date: "2026-02-28".to_string(),
            notes: String::new(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let response = client
        .create_invoice(request)
        .await
        .expect("Failed to create invoice");
    response
        .into_inner()
        .invoice
        .expect("Missing invoice")
        .invoice_id
}

/// Helper to add a line item to an invoice.
async fn add_line_item(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
    invoice_id: &str,
    description: &str,
    quantity: &str,
    unit_price: &str,
) {
    let request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.to_string(),
            description: description.to_string(),
            quantity: quantity.to_string(),
            unit_price: unit_price.to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    client
        .add_line_item(request)
        .await
        .expect("Failed to add line item");
}

#[tokio::test]
async fn issue_invoice_transitions_to_issued() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create draft invoice with line item
    let invoice_id = create_draft_invoice(&mut client, "Issue Test Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    // Issue the invoice
    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    let issue_response = client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");
    let issued_invoice = issue_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    assert_eq!(issued_invoice.status, InvoiceStatus::Issued as i32);
    assert!(
        !issued_invoice.invoice_number.is_empty(),
        "Invoice should have a number"
    );
    assert_eq!(issued_invoice.issue_date, "2026-01-23");

    app.cleanup().await;
}

#[tokio::test]
async fn issue_invoice_assigns_sequential_number() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create and issue two invoices
    let invoice_id1 = create_draft_invoice(&mut client, "Sequential Test 1").await;
    add_line_item(&mut client, &invoice_id1, "Service 1", "1", "50.00").await;

    let invoice_id2 = create_draft_invoice(&mut client, "Sequential Test 2").await;
    add_line_item(&mut client, &invoice_id2, "Service 2", "1", "75.00").await;

    // Issue first invoice
    let issue_request1 = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id1.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    let response1 = client
        .issue_invoice(issue_request1)
        .await
        .expect("Failed to issue first invoice");
    let invoice1 = response1.into_inner().invoice.expect("Missing invoice");

    // Issue second invoice
    let issue_request2 = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id2.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    let response2 = client
        .issue_invoice(issue_request2)
        .await
        .expect("Failed to issue second invoice");
    let invoice2 = response2.into_inner().invoice.expect("Missing invoice");

    // Invoice numbers should be different (sequential)
    assert_ne!(invoice1.invoice_number, invoice2.invoice_number);

    app.cleanup().await;
}

#[tokio::test]
async fn issue_empty_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create draft invoice WITHOUT line items
    let invoice_id = create_draft_invoice(&mut client, "Empty Invoice Customer").await;

    // Try to issue the invoice
    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    let result = client.issue_invoice(issue_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn issue_already_issued_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create and issue an invoice
    let invoice_id = create_draft_invoice(&mut client, "Double Issue Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    // Try to issue again
    let issue_request2 = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-24".to_string(),
        },
    );

    let result = client.issue_invoice(issue_request2).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn void_issued_invoice_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create, add line item, and issue an invoice
    let invoice_id = create_draft_invoice(&mut client, "Void Test Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    // Void the invoice
    let void_request = with_tenant(
        TEST_TENANT_ID,
        VoidInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            reason: "Customer cancelled".to_string(),
        },
    );

    let void_response = client
        .void_invoice(void_request)
        .await
        .expect("Failed to void invoice");
    let voided_invoice = void_response.into_inner().invoice.expect("Missing invoice");

    assert_eq!(voided_invoice.status, InvoiceStatus::Void as i32);

    app.cleanup().await;
}

#[tokio::test]
async fn void_draft_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create draft invoice (don't issue it)
    let invoice_id = create_draft_invoice(&mut client, "Void Draft Customer").await;

    // Try to void it
    let void_request = with_tenant(
        TEST_TENANT_ID,
        VoidInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            reason: "Try to void draft".to_string(),
        },
    );

    let result = client.void_invoice(void_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn void_already_voided_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create, issue, and void an invoice
    let invoice_id = create_draft_invoice(&mut client, "Double Void Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    let void_request = with_tenant(
        TEST_TENANT_ID,
        VoidInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            reason: "First void".to_string(),
        },
    );

    client
        .void_invoice(void_request)
        .await
        .expect("Failed to void invoice");

    // Try to void again
    let void_request2 = with_tenant(
        TEST_TENANT_ID,
        VoidInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            reason: "Second void attempt".to_string(),
        },
    );

    let result = client.void_invoice(void_request2).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn issued_invoice_cannot_be_updated() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create, add line item, and issue an invoice
    let invoice_id = create_draft_invoice(&mut client, "Update Issued Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    // Try to update the issued invoice
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            customer_name: "New Name".to_string(),
            billing_address: None,
            due_date: String::new(),
            notes: String::new(),
            metadata: String::new(),
        },
    );

    let result = client.update_invoice(update_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn issued_invoice_cannot_be_deleted() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create, add line item, and issue an invoice
    let invoice_id = create_draft_invoice(&mut client, "Delete Issued Customer").await;
    add_line_item(&mut client, &invoice_id, "Test Service", "1", "100.00").await;

    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    // Try to delete the issued invoice
    let delete_request = with_tenant(
        TEST_TENANT_ID,
        invoicing_service::grpc::proto::DeleteInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
        },
    );

    let result = client.delete_invoice(delete_request).await;

    // Should either return error or success=false
    if let Ok(response) = result {
        assert!(
            !response.into_inner().success,
            "Delete should not succeed for issued invoice"
        );
    }
    // If error, that's also acceptable behavior

    app.cleanup().await;
}
