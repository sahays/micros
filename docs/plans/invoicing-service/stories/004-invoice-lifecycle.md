# Story: Invoice Lifecycle

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement IssueInvoice and VoidInvoice gRPC methods with ledger-service integration for accounting entries.

## Tasks

- [ ] Define proto messages: IssueInvoiceRequest/Response, VoidInvoiceRequest/Response
- [ ] Implement IssueInvoice handler
- [ ] Implement VoidInvoice handler
- [ ] Integrate with ledger-service client
- [ ] Implement journal entry creation for issue
- [ ] Implement reversing entry creation for void
- [ ] Handle ledger-service failures with rollback

## gRPC Methods

### IssueInvoice
**Input:** tenant_id, invoice_id, issue_date (optional, defaults to today)
**Output:** invoice, journal_id

**Behavior:**
1. Validate invoice is in draft status
2. Validate invoice has at least one line item
3. Assign invoice_number (next in sequence)
4. Set issue_date and status to issued
5. Create ledger journal entry
6. Return issued invoice with journal_id

**Ledger Entry:**
- Debit: Accounts Receivable (customer) for total
- Credit: Revenue accounts (per line item) for subtotal
- Credit: Tax Payable for tax_total

### VoidInvoice
**Input:** tenant_id, invoice_id, reason
**Output:** invoice, journal_id

**Behavior:**
1. Validate invoice is in issued status (not paid)
2. Set status to void and voided_utc
3. Create reversing ledger journal entry
4. Return voided invoice with journal_id

**Validation:**
- Cannot void fully or partially paid invoices
- Cannot void already voided invoices

## Acceptance Criteria

- [ ] IssueInvoice transitions draft to issued
- [ ] IssueInvoice assigns invoice_number
- [ ] IssueInvoice creates correct ledger entry
- [ ] IssueInvoice rejects empty invoices
- [ ] IssueInvoice rejects non-draft invoices
- [ ] VoidInvoice transitions issued to void
- [ ] VoidInvoice creates reversing ledger entry
- [ ] VoidInvoice rejects paid invoices
- [ ] Ledger failure rolls back invoice status change

## Integration Tests

- [ ] Issue draft invoice succeeds with ledger entry
- [ ] Issue invoice assigns sequential number
- [ ] Issue empty invoice returns FAILED_PRECONDITION
- [ ] Void issued invoice succeeds with reversing entry
- [ ] Void paid invoice returns FAILED_PRECONDITION
- [ ] Void draft invoice returns FAILED_PRECONDITION
