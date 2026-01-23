//! Line item integration tests for invoicing-service.

mod common;

use common::{with_tenant, TestApp, TEST_CUSTOMER_ID, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    AddLineItemRequest, Address, CreateInvoiceRequest, GetInvoiceRequest, InvoiceType,
    IssueInvoiceRequest, RemoveLineItemRequest, UpdateLineItemRequest,
};

/// Helper to create a billing address for tests.
fn test_address() -> Option<Address> {
    Some(Address {
        line1: "123 Line Street".to_string(),
        line2: String::new(),
        city: "Line City".to_string(),
        state: "LI".to_string(),
        postal_code: "12345".to_string(),
        country: "US".to_string(),
    })
}

/// Helper to create a draft invoice for line item testing.
async fn create_draft_invoice(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
) -> String {
    let request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Line Item Test Customer".to_string(),
            billing_address: test_address(),
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

#[tokio::test]
async fn add_line_item_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Consulting Services".to_string(),
            quantity: "10".to_string(),
            unit_price: "150.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let add_response = client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");
    let line_item = add_response
        .into_inner()
        .line_item
        .expect("Missing line item");

    assert!(!line_item.line_item_id.is_empty());
    assert_eq!(line_item.description, "Consulting Services");
    assert_eq!(line_item.quantity, "10");
    assert_eq!(line_item.unit_price, "150");
    assert_eq!(line_item.subtotal, "1500"); // 10 * 150
    assert_eq!(line_item.total, "1500");

    app.cleanup().await;
}

#[tokio::test]
async fn add_multiple_line_items() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add first line item
    let add_request1 = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Product A".to_string(),
            quantity: "5".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 1,
        },
    );

    client
        .add_line_item(add_request1)
        .await
        .expect("Failed to add first line item");

    // Add second line item
    let add_request2 = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Product B".to_string(),
            quantity: "3".to_string(),
            unit_price: "200.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 2,
        },
    );

    client
        .add_line_item(add_request2)
        .await
        .expect("Failed to add second line item");

    // Get invoice to verify totals
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
        },
    );

    let get_response = client
        .get_invoice(get_request)
        .await
        .expect("Failed to get invoice");
    let invoice = get_response.into_inner().invoice.expect("Missing invoice");

    assert_eq!(invoice.line_items.len(), 2);
    // Total should be (5*100) + (3*200) = 500 + 600 = 1100
    assert_eq!(invoice.subtotal, "1100");
    assert_eq!(invoice.total, "1100");

    app.cleanup().await;
}

#[tokio::test]
async fn update_line_item_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add a line item
    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Original Description".to_string(),
            quantity: "1".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let add_response = client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");
    let line_item_id = add_response
        .into_inner()
        .line_item
        .expect("Missing line item")
        .line_item_id;

    // Update the line item
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            line_item_id: line_item_id.clone(),
            description: "Updated Description".to_string(),
            quantity: "5".to_string(),
            unit_price: "200.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 1,
        },
    );

    let update_response = client
        .update_line_item(update_request)
        .await
        .expect("Failed to update line item");
    let updated_item = update_response
        .into_inner()
        .line_item
        .expect("Missing line item");

    assert_eq!(updated_item.description, "Updated Description");
    assert_eq!(updated_item.quantity, "5");
    assert_eq!(updated_item.unit_price, "200");
    assert_eq!(updated_item.subtotal, "1000"); // 5 * 200
    assert_eq!(updated_item.total, "1000");

    app.cleanup().await;
}

#[tokio::test]
async fn remove_line_item_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add two line items
    let add_request1 = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Item to Keep".to_string(),
            quantity: "1".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 1,
        },
    );

    client
        .add_line_item(add_request1)
        .await
        .expect("Failed to add first line item");

    let add_request2 = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Item to Remove".to_string(),
            quantity: "1".to_string(),
            unit_price: "50.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 2,
        },
    );

    let add_response2 = client
        .add_line_item(add_request2)
        .await
        .expect("Failed to add second line item");
    let line_item_id = add_response2
        .into_inner()
        .line_item
        .expect("Missing line item")
        .line_item_id;

    // Remove the second line item
    let remove_request = with_tenant(
        TEST_TENANT_ID,
        RemoveLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            line_item_id: line_item_id.clone(),
        },
    );

    let remove_response = client
        .remove_line_item(remove_request)
        .await
        .expect("Failed to remove line item");
    let updated_invoice = remove_response
        .into_inner()
        .invoice
        .expect("Missing invoice");

    assert_eq!(updated_invoice.line_items.len(), 1);
    assert_eq!(updated_invoice.line_items[0].description, "Item to Keep");
    assert_eq!(updated_invoice.subtotal, "100");

    app.cleanup().await;
}

#[tokio::test]
async fn add_line_item_to_issued_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add initial line item
    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Initial Item".to_string(),
            quantity: "1".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");

    // Issue the invoice
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

    // Try to add another line item
    let add_request2 = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "New Item After Issue".to_string(),
            quantity: "1".to_string(),
            unit_price: "50.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 1,
        },
    );

    let result = client.add_line_item(add_request2).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn update_line_item_on_issued_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add line item
    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Test Item".to_string(),
            quantity: "1".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let add_response = client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");
    let line_item_id = add_response
        .into_inner()
        .line_item
        .expect("Missing line item")
        .line_item_id;

    // Issue the invoice
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

    // Try to update the line item
    let update_request = with_tenant(
        TEST_TENANT_ID,
        UpdateLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            line_item_id: line_item_id.clone(),
            description: "Updated After Issue".to_string(),
            quantity: "2".to_string(),
            unit_price: "150.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let result = client.update_line_item(update_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn remove_line_item_from_issued_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    // Add line item
    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Test Item".to_string(),
            quantity: "1".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let add_response = client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");
    let line_item_id = add_response
        .into_inner()
        .line_item
        .expect("Missing line item")
        .line_item_id;

    // Issue the invoice
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

    // Try to remove the line item
    let remove_request = with_tenant(
        TEST_TENANT_ID,
        RemoveLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            line_item_id: line_item_id.clone(),
        },
    );

    let result = client.remove_line_item(remove_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn line_item_with_fractional_quantity() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_draft_invoice(&mut client).await;

    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Hourly Consulting".to_string(),
            quantity: "2.5".to_string(),
            unit_price: "100.00".to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    let add_response = client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");
    let line_item = add_response
        .into_inner()
        .line_item
        .expect("Missing line item");

    assert_eq!(line_item.quantity, "2.5");
    assert_eq!(line_item.subtotal, "250"); // 2.5 * 100

    app.cleanup().await;
}
