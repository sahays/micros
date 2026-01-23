# Story: Invoice Lifecycle

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement IssueInvoice and VoidInvoice gRPC methods with ledger-service integration for accounting entries.

## Tasks

- [x] Define proto messages: IssueInvoiceRequest/Response, VoidInvoiceRequest/Response
- [x] Implement IssueInvoice handler
- [x] Implement VoidInvoice handler
- [x] Integrate with ledger-service client
- [x] Implement journal entry creation for issue
- [x] Implement reversing entry creation for void
- [x] Handle ledger-service failures with rollback

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

**Status:** ✅ Implemented

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

**Status:** ✅ Implemented

## Acceptance Criteria

- [x] IssueInvoice transitions draft to issued
- [x] IssueInvoice assigns invoice_number
- [x] IssueInvoice creates correct ledger entry
- [x] IssueInvoice rejects empty invoices
- [x] IssueInvoice rejects non-draft invoices
- [x] VoidInvoice transitions issued to void
- [x] VoidInvoice creates reversing ledger entry
- [x] VoidInvoice rejects paid invoices
- [x] Ledger failure rolls back invoice status change

## Integration Tests

- [x] Issue draft invoice succeeds with ledger entry
- [x] Issue invoice assigns sequential number
- [x] Issue empty invoice returns FAILED_PRECONDITION
- [x] Issue already-issued invoice returns FAILED_PRECONDITION
- [x] Void issued invoice succeeds with reversing entry
- [x] Void paid invoice returns FAILED_PRECONDITION
- [x] Void draft invoice returns FAILED_PRECONDITION
- [x] Void already-voided invoice returns FAILED_PRECONDITION
