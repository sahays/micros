# Story: Invoice Management

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement CreateInvoice, GetInvoice, UpdateInvoice, ListInvoices, and DeleteInvoice gRPC methods for draft invoice management.

## Tasks

- [x] Define proto messages: Invoice, CreateInvoiceRequest/Response
- [x] Define proto messages: GetInvoiceRequest/Response
- [x] Define proto messages: UpdateInvoiceRequest/Response
- [x] Define proto messages: ListInvoicesRequest/Response
- [x] Define proto messages: DeleteInvoiceRequest/Response
- [x] Implement CreateInvoice handler with validation
- [x] Implement GetInvoice handler
- [x] Implement UpdateInvoice handler (draft only)
- [x] Implement ListInvoices handler with filters and pagination
- [x] Implement DeleteInvoice handler (draft only)
- [x] Implement invoice number generation (INV-YYYYMM-NNNN)
- [x] Add tenant context extraction from gRPC metadata

## gRPC Methods

### CreateInvoice
**Input:** tenant_id, invoice_type, customer_id, customer_name, billing_address, currency, due_date, notes, reference_invoice_id, metadata
**Output:** invoice (in draft status)

**Validation:**
- currency is valid (non-empty)
- customer_id is valid UUID
- reference_invoice_id required for credit notes

**Status:** ✅ Implemented

### GetInvoice
**Input:** tenant_id, invoice_id
**Output:** invoice with line items

**Status:** ✅ Implemented

### UpdateInvoice
**Input:** tenant_id, invoice_id, customer_name, billing_address, due_date, notes, metadata
**Output:** invoice

**Validation:**
- Invoice must be in draft status
- Cannot update invoice_number or currency

**Status:** ✅ Implemented

### ListInvoices
**Input:** tenant_id, status (optional), customer_id (optional), start_date (optional), end_date (optional), page_size, page_token
**Output:** invoices[], next_page_token

**Status:** ✅ Implemented

### DeleteInvoice
**Input:** tenant_id, invoice_id
**Output:** success boolean

**Validation:**
- Only draft invoices can be deleted

**Status:** ✅ Implemented

## Acceptance Criteria

- [x] CreateInvoice returns new invoice in draft status
- [x] GetInvoice returns invoice with all line items
- [x] GetInvoice returns NOT_FOUND for missing invoice
- [x] UpdateInvoice modifies draft invoice
- [x] UpdateInvoice rejects updates to issued invoices
- [x] DeleteInvoice removes draft invoice
- [x] DeleteInvoice rejects deletion of issued invoices
- [x] ListInvoices filters by status, customer, date range
- [x] ListInvoices pagination works correctly
- [x] All methods enforce tenant isolation

## Integration Tests

- [x] Create invoice with valid data returns invoice in draft status
- [x] Get invoice returns complete invoice with items
- [x] Update draft invoice succeeds
- [x] Update issued invoice returns FAILED_PRECONDITION
- [x] Delete draft invoice succeeds
- [x] Delete issued invoice returns FAILED_PRECONDITION
- [x] List invoices returns only tenant's invoices
- [x] List invoices with filters returns matching subset
