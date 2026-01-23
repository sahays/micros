# Story: Payments and Receipts

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement RecordPayment, GetReceipt, and ListReceipts gRPC methods for payment processing and receipt generation.

## Tasks

- [ ] Define proto messages: Receipt, RecordPaymentRequest/Response
- [ ] Define proto messages: GetReceiptRequest/Response, ListReceiptsRequest/Response
- [ ] Implement RecordPayment handler with ledger integration
- [ ] Implement GetReceipt handler
- [ ] Implement ListReceipts handler with filters
- [ ] Implement receipt number generation
- [ ] Implement invoice status update on payment
- [ ] Handle partial payments

## gRPC Methods

### RecordPayment
**Input:** tenant_id, invoice_id, amount, payment_method, payment_reference, received_date, notes
**Output:** receipt, journal_id, updated invoice

**Behavior:**
1. Validate invoice is issued or partially paid
2. Validate amount <= balance_due
3. Create receipt with generated receipt_number
4. Create ledger journal entry
5. Update invoice amount_paid and balance_due
6. Update invoice status (paid if balance_due = 0)

**Ledger Entry:**
- Debit: Cash/Bank Account for amount
- Credit: Accounts Receivable (customer) for amount

### GetReceipt
**Input:** tenant_id, receipt_id
**Output:** receipt

### ListReceipts
**Input:** tenant_id, invoice_id (optional), date_from (optional), date_to (optional), page_size, page_token
**Output:** receipts[], next_page_token

## Payment Methods

Supported values (stored as string, not enum):
- cash
- card
- bank_transfer
- check
- upi
- other

## Acceptance Criteria

- [ ] RecordPayment creates receipt with generated number
- [ ] RecordPayment creates correct ledger entry
- [ ] RecordPayment updates invoice amount_paid
- [ ] RecordPayment sets status to paid when fully paid
- [ ] RecordPayment allows partial payments
- [ ] RecordPayment rejects overpayment
- [ ] RecordPayment rejects payment on draft/void invoice
- [ ] GetReceipt returns receipt details
- [ ] ListReceipts filters by invoice_id and date range

## Integration Tests

- [ ] Record full payment marks invoice as paid
- [ ] Record partial payment updates balance_due
- [ ] Record multiple partial payments succeeds
- [ ] Record payment exceeding balance returns INVALID_ARGUMENT
- [ ] Record payment on draft invoice returns FAILED_PRECONDITION
- [ ] List receipts for invoice returns all payments
