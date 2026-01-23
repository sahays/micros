//! Payment and receipt integration tests for invoicing-service.

mod common;

use common::{with_tenant, TestApp, TEST_CUSTOMER_ID, TEST_TENANT_ID};
use invoicing_service::grpc::proto::{
    AddLineItemRequest, Address, CreateInvoiceRequest, GetReceiptRequest, InvoiceStatus,
    InvoiceType, IssueInvoiceRequest, ListReceiptsRequest, RecordPaymentRequest,
};

/// Helper to create a billing address for tests.
fn test_address() -> Option<Address> {
    Some(Address {
        line1: "123 Payment Street".to_string(),
        line2: String::new(),
        city: "Payment City".to_string(),
        state: "PC".to_string(),
        postal_code: "12345".to_string(),
        country: "US".to_string(),
    })
}

/// Helper to create and issue an invoice for payment testing.
async fn create_issued_invoice(
    client: &mut invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient<
        tonic::transport::Channel,
    >,
    customer_name: &str,
    amount: &str,
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
            issue_date: "2026-01-23".to_string(),
        },
    );

    client
        .issue_invoice(issue_request)
        .await
        .expect("Failed to issue invoice");

    invoice_id
}

#[tokio::test]
async fn record_full_payment_marks_invoice_as_paid() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Full Payment Customer", "100.00").await;

    // Record full payment
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "card".to_string(),
            payment_reference: "TXN-123456".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: "Full payment received".to_string(),
        },
    );

    let payment_response = client
        .record_payment(payment_request)
        .await
        .expect("Failed to record payment");
    let response = payment_response.into_inner();

    let receipt = response.receipt.expect("Missing receipt");
    assert!(!receipt.receipt_id.is_empty());
    assert!(!receipt.receipt_number.is_empty());
    assert_eq!(receipt.amount, "100");
    assert_eq!(receipt.payment_method, "card");

    let invoice = response.invoice.expect("Missing invoice");
    assert_eq!(invoice.status, InvoiceStatus::Paid as i32);
    assert_eq!(invoice.amount_paid, "100");
    assert_eq!(invoice.amount_due, "0");

    app.cleanup().await;
}

#[tokio::test]
async fn record_partial_payment_updates_balance() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Partial Payment Customer", "200.00").await;

    // Record partial payment
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "75.00".to_string(),
            payment_method: "bank_transfer".to_string(),
            payment_reference: "BANK-789".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: "Partial payment".to_string(),
        },
    );

    let payment_response = client
        .record_payment(payment_request)
        .await
        .expect("Failed to record payment");
    let response = payment_response.into_inner();

    let invoice = response.invoice.expect("Missing invoice");
    assert_eq!(invoice.status, InvoiceStatus::Issued as i32); // Still issued, not fully paid
    assert_eq!(invoice.amount_paid, "75");
    assert_eq!(invoice.amount_due, "125");

    app.cleanup().await;
}

#[tokio::test]
async fn record_multiple_partial_payments_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Multi Payment Customer", "300.00").await;

    // First payment
    let payment1 = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "cash".to_string(),
            payment_reference: "CASH-001".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: "First installment".to_string(),
        },
    );

    let response1 = client
        .record_payment(payment1)
        .await
        .expect("Failed to record first payment");
    let inv1 = response1.into_inner().invoice.expect("Missing invoice");
    assert_eq!(inv1.amount_due, "200");

    // Second payment
    let payment2 = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "cash".to_string(),
            payment_reference: "CASH-002".to_string(),
            payment_date: "2026-01-26".to_string(),
            notes: "Second installment".to_string(),
        },
    );

    let response2 = client
        .record_payment(payment2)
        .await
        .expect("Failed to record second payment");
    let inv2 = response2.into_inner().invoice.expect("Missing invoice");
    assert_eq!(inv2.amount_due, "100");

    // Final payment
    let payment3 = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "cash".to_string(),
            payment_reference: "CASH-003".to_string(),
            payment_date: "2026-01-27".to_string(),
            notes: "Final installment".to_string(),
        },
    );

    let response3 = client
        .record_payment(payment3)
        .await
        .expect("Failed to record final payment");
    let inv3 = response3.into_inner().invoice.expect("Missing invoice");
    assert_eq!(inv3.status, InvoiceStatus::Paid as i32);
    assert_eq!(inv3.amount_due, "0");
    assert_eq!(inv3.amount_paid, "300");

    app.cleanup().await;
}

#[tokio::test]
async fn record_overpayment_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Overpayment Customer", "100.00").await;

    // Try to pay more than owed
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "150.00".to_string(),
            payment_method: "card".to_string(),
            payment_reference: "OVER-001".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: String::new(),
        },
    );

    let result = client.record_payment(payment_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    // FailedPrecondition because the invoice balance_due is a precondition
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn record_payment_on_draft_invoice_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Create draft invoice WITHOUT issuing it
    let create_request = with_tenant(
        TEST_TENANT_ID,
        CreateInvoiceRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_type: InvoiceType::Standard as i32,
            customer_id: TEST_CUSTOMER_ID.to_string(),
            customer_name: "Draft Payment Customer".to_string(),
            billing_address: test_address(),
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
    let invoice_id = create_response
        .into_inner()
        .invoice
        .expect("Missing invoice")
        .invoice_id;

    // Try to record payment on draft
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "card".to_string(),
            payment_reference: "DRAFT-001".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: String::new(),
        },
    );

    let result = client.record_payment(payment_request).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn get_receipt_returns_payment_details() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Get Receipt Customer", "100.00").await;

    // Record payment
    let payment_request = with_tenant(
        TEST_TENANT_ID,
        RecordPaymentRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            amount: "100.00".to_string(),
            payment_method: "upi".to_string(),
            payment_reference: "UPI-123".to_string(),
            payment_date: "2026-01-25".to_string(),
            notes: "Payment via UPI".to_string(),
        },
    );

    let payment_response = client
        .record_payment(payment_request)
        .await
        .expect("Failed to record payment");
    let receipt_id = payment_response
        .into_inner()
        .receipt
        .expect("Missing receipt")
        .receipt_id;

    // Get receipt
    let get_request = with_tenant(
        TEST_TENANT_ID,
        GetReceiptRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            receipt_id: receipt_id.clone(),
        },
    );

    let get_response = client
        .get_receipt(get_request)
        .await
        .expect("Failed to get receipt");
    let receipt = get_response.into_inner().receipt.expect("Missing receipt");

    assert_eq!(receipt.receipt_id, receipt_id);
    assert_eq!(receipt.amount, "100");
    assert_eq!(receipt.payment_method, "upi");
    assert_eq!(receipt.payment_reference, "UPI-123");
    assert_eq!(receipt.notes, "Payment via UPI");

    app.cleanup().await;
}

#[tokio::test]
async fn list_receipts_for_invoice_returns_all_payments() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "List Receipts Customer", "300.00").await;

    // Record multiple payments
    for i in 1..=3 {
        let payment_request = with_tenant(
            TEST_TENANT_ID,
            RecordPaymentRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                invoice_id: invoice_id.clone(),
                amount: "100.00".to_string(),
                payment_method: "check".to_string(),
                payment_reference: format!("CHECK-{:03}", i),
                payment_date: format!("2026-01-{:02}", 24 + i),
                notes: format!("Check payment {}", i),
            },
        );

        client
            .record_payment(payment_request)
            .await
            .expect("Failed to record payment");
    }

    // List receipts for this invoice
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListReceiptsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: invoice_id.clone(),
            customer_id: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            page_size: 10,
            page_token: String::new(),
        },
    );

    let list_response = client
        .list_receipts(list_request)
        .await
        .expect("Failed to list receipts");
    let receipts = list_response.into_inner().receipts;

    assert_eq!(receipts.len(), 3);
    for receipt in &receipts {
        assert_eq!(receipt.invoice_id, invoice_id);
        assert_eq!(receipt.amount, "100");
    }

    app.cleanup().await;
}

#[tokio::test]
async fn list_receipts_by_date_range() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let invoice_id = create_issued_invoice(&mut client, "Date Range Customer", "500.00").await;

    // Record payments on different dates
    let dates = ["2026-01-20", "2026-01-25", "2026-02-01"];
    for (i, date) in dates.iter().enumerate() {
        let payment_request = with_tenant(
            TEST_TENANT_ID,
            RecordPaymentRequest {
                tenant_id: TEST_TENANT_ID.to_string(),
                invoice_id: invoice_id.clone(),
                amount: "100.00".to_string(),
                payment_method: "card".to_string(),
                payment_reference: format!("CARD-{:03}", i + 1),
                payment_date: date.to_string(),
                notes: String::new(),
            },
        );

        client
            .record_payment(payment_request)
            .await
            .expect("Failed to record payment");
    }

    // List receipts for January only
    let list_request = with_tenant(
        TEST_TENANT_ID,
        ListReceiptsRequest {
            tenant_id: TEST_TENANT_ID.to_string(),
            invoice_id: String::new(),
            customer_id: String::new(),
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-31".to_string(),
            page_size: 10,
            page_token: String::new(),
        },
    );

    let list_response = client
        .list_receipts(list_request)
        .await
        .expect("Failed to list receipts");
    let receipts = list_response.into_inner().receipts;

    // Should only include receipts from January (2 out of 3)
    assert_eq!(receipts.len(), 2);

    app.cleanup().await;
}
