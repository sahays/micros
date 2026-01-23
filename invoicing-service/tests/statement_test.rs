//! Statement generation integration tests for invoicing-service.

mod common;

use common::{with_tenant, TestApp, TEST_CUSTOMER_ID, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    AddLineItemRequest, Address, CreateInvoiceRequest, GenerateStatementRequest, InvoiceType,
    IssueInvoiceRequest, RecordPaymentRequest,
};

/// Helper to create a billing address for tests.
fn test_address() -> Option<Address> {
    Some(Address {
        line1: "123 Statement Street".to_string(),
        line2: String::new(),
        city: "Statement City".to_string(),
        state: "ST".to_string(),
        postal_code: "12345".to_string(),
        country: "US".to_string(),
    })
}

/// Helper to create and issue an invoice.
async fn create_and_issue_invoice(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
    customer_name: &str,
    amount: &str,
    issue_date: &str,
) -> String {
    // Create draft
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: customer_name.to_string(),
            billing_address: test_address(),
            currency: "USD".to_string(),
            due_date: "2026-03-31".to_string(),
            notes: String::new(),
            reference_invoice_id: String::new(),
            metadata: "{}".to_string(),
        },
    );

    let create_response = client
        .create_invoice(create_request)
        .await
        .expect("Failed to create invoice");
    let invoice_id = create_response
        .into_inner()
        .invoice
        .expect("Missing invoice")
        .invoice_id;

    // Add line item
    let add_request = with_tenant(
        TEST_TENANT_ID,
        AddLineItemRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            description: "Test Service".to_string(),
            quantity: "1".to_string(),
            unit_price: amount.to_string(),
            tax_rate_id: String::new(),
            ledger_account_id: String::new(),
            sort_order: 0,
        },
    );

    client
        .add_line_item(add_request)
        .await
        .expect("Failed to add line item");

    // Issue
    let issue_request = with_tenant(
        TEST_TENANT_ID,
        IssueInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            issue_date: issue_date.to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    invoice_id
}

/// Helper to record a payment.
async fn record_payment(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
    invoice_id: &str,
    amount: &str,
    payment_date: &str,
) {
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.to_string(),
            amount: amount.to_string(),
            payment_method: "card".to_string(),
            payment_reference: format!("PAY-{}", uuid::Uuid::new_v4()),
            payment_date: payment_date.to_string(),
            notes: String::new(),
        },
    );

    client
        .record_payment(payment_request)
        .await
        .expect("Failed to record payment");
}

#[tokio::test]
async fn generate_statement_with_no_activity() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Generate statement for customer with no invoices returns NotFound
    // (customer has no invoicing history yet)
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let result = client.generate_statement(statement_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_with_invoices_only() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create and issue invoices in the period
    create_and_issue_invoice(&mut client, "Statement Customer", "100.00", "2026-01-15").await;
    create_and_issue_invoice(&mut client, "Statement Customer", "200.00", "2026-01-20").await;

    // Generate statement
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let statement_response = client
        .generate_statement(statement_request)
        .await
        .expect("Failed to generate statement");
    let statement = statement_response
        .into_inner()
        .statement
        .expect("Missing statement");

    assert_eq!(statement.opening_balance, "0");
    assert_eq!(statement.closing_balance, "300"); // 100 + 200
    assert_eq!(statement.lines.len(), 2);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_with_payments() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create and issue invoice
    let invoice_id = create_and_issue_invoice(
        &mut client,
        "Payment Statement Customer",
        "300.00",
        "2026-01-10",
    )
    .await;

    // Record partial payment
    record_payment(&mut client, &invoice_id, "150.00", "2026-01-20").await;

    // Generate statement
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let statement_response = client
        .generate_statement(statement_request)
        .await
        .expect("Failed to generate statement");
    let statement = statement_response
        .into_inner()
        .statement
        .expect("Missing statement");

    assert_eq!(statement.opening_balance, "0");
    // Closing balance = 300 (invoice) - 150 (payment) = 150
    assert_eq!(statement.closing_balance, "150");

    // Should have 2 lines: invoice and payment
    assert_eq!(statement.lines.len(), 2);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_with_opening_balance() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create invoice BEFORE the statement period
    create_and_issue_invoice(
        &mut client,
        "Opening Balance Customer",
        "500.00",
        "2025-12-15",
    )
    .await;

    // Create invoice IN the statement period
    create_and_issue_invoice(
        &mut client,
        "Opening Balance Customer",
        "200.00",
        "2026-01-15",
    )
    .await;

    // Generate statement for January
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let statement_response = client
        .generate_statement(statement_request)
        .await
        .expect("Failed to generate statement");
    let statement = statement_response
        .into_inner()
        .statement
        .expect("Missing statement");

    // Opening balance should include the December invoice
    assert_eq!(statement.opening_balance, "500");
    // Closing balance = opening (500) + January invoice (200) = 700
    assert_eq!(statement.closing_balance, "700");

    // Only January invoice should be in lines
    assert_eq!(statement.lines.len(), 1);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_running_balance() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create multiple invoices and payments to test running balance
    let inv1 = create_and_issue_invoice(
        &mut client,
        "Running Balance Customer",
        "100.00",
        "2026-01-05",
    )
    .await;
    let inv2 = create_and_issue_invoice(
        &mut client,
        "Running Balance Customer",
        "200.00",
        "2026-01-10",
    )
    .await;
    record_payment(&mut client, &inv1, "100.00", "2026-01-15").await;
    let _inv3 = create_and_issue_invoice(
        &mut client,
        "Running Balance Customer",
        "150.00",
        "2026-01-20",
    )
    .await;
    record_payment(&mut client, &inv2, "100.00", "2026-01-25").await;

    // Generate statement
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let statement_response = client
        .generate_statement(statement_request)
        .await
        .expect("Failed to generate statement");
    let statement = statement_response
        .into_inner()
        .statement
        .expect("Missing statement");

    assert_eq!(statement.opening_balance, "0");
    // Total: 100 + 200 - 100 + 150 - 100 = 250
    assert_eq!(statement.closing_balance, "250");

    // Should have 5 lines (3 invoices + 2 payments)
    assert_eq!(statement.lines.len(), 5);

    // Verify running balance is tracked correctly
    // Each line should have a running_balance field
    let mut running = rust_decimal::Decimal::ZERO;
    for line in &statement.lines {
        if line.document_type == "invoice" {
            running += rust_decimal::Decimal::from_str_exact(&line.debit).unwrap_or_default();
        } else if line.document_type == "payment" {
            running -= rust_decimal::Decimal::from_str_exact(&line.credit).unwrap_or_default();
        }
    }

    // Final running balance should match closing balance
    assert_eq!(running.to_string(), statement.closing_balance);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_invalid_date_range() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Try to generate statement with end_date before start_date
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: TEST_CUSTOMER_ID.to_string(),
            period_start: "2026-01-31".to_string(),
            period_end: "2026-01-01".to_string(), // Before start
        },
    );

    let result = client.generate_statement(statement_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);

    app.cleanup().await;
}

#[tokio::test]
async fn generate_statement_invalid_customer_id() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Try to generate statement with invalid customer ID
    let statement_request = with_tenant(
        TEST_TENANT_ID,
        GenerateStatementRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            customer_id: "invalid-uuid".to_string(),
            period_start: "2026-01-01".to_string(),
            period_end: "2026-01-31".to_string(),
        },
    );

    let result = client.generate_statement(statement_request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);

    app.cleanup().await;
}
