# Story: Invoice Management

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement CreateInvoice, GetInvoice, UpdateInvoice, and ListInvoices gRPC methods for draft invoice management.

## Tasks

- [ ] Define proto messages: Invoice, CreateInvoiceRequest/Response
- [ ] Define proto messages: GetInvoiceRequest/Response, UpdateInvoiceRequest/Response
- [ ] Define proto messages: ListInvoicesRequest/Response
- [ ] Implement CreateInvoice handler with validation
- [ ] Implement GetInvoice handler
- [ ] Implement UpdateInvoice handler (draft only)
- [ ] Implement ListInvoices handler with filters and pagination
- [ ] Implement invoice number generation (INV-YYYYMM-NNNN)
- [ ] Add tenant context extraction from gRPC metadata

## gRPC Methods

### CreateInvoice
**Input:** tenant_id, customer_id, customer_name, customer_address, currency, due_date, items[], notes, terms, metadata
**Output:** invoice

**Validation:**
- currency is valid ISO 4217
- due_date is in the future
- customer_id is valid UUID

### GetInvoice
**Input:** tenant_id, invoice_id
**Output:** invoice with line items

### UpdateInvoice
**Input:** tenant_id, invoice_id, customer_name, customer_address, due_date, notes, terms, metadata
**Output:** invoice

**Validation:**
- Invoice must be in draft status
- Cannot update invoice_number or currency

### ListInvoices
**Input:** tenant_id, status (optional), customer_id (optional), date_from (optional), date_to (optional), page_size, page_token
**Output:** invoices[], next_page_token

## Acceptance Criteria

- [ ] CreateInvoice returns new invoice with generated number
- [ ] CreateInvoice creates invoice in draft status
- [ ] GetInvoice returns invoice with all line items
- [ ] GetInvoice returns NOT_FOUND for missing invoice
- [ ] UpdateInvoice modifies draft invoice
- [ ] UpdateInvoice rejects updates to issued invoices
- [ ] ListInvoices filters by status, customer, date range
- [ ] ListInvoices pagination works correctly
- [ ] All methods enforce tenant isolation

## Integration Tests

- [ ] Create invoice with valid data returns invoice in draft status
- [ ] Get invoice returns complete invoice with items
- [ ] Update draft invoice succeeds
- [ ] Update issued invoice returns FAILED_PRECONDITION
- [ ] List invoices returns only tenant's invoices
- [ ] List invoices with filters returns matching subset
