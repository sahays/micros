# Story: Payments and Receipts

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement RecordPayment, GetReceipt, and ListReceipts gRPC methods for payment processing and receipt generation.

## Tasks

- [x] Define proto messages: Receipt, RecordPaymentRequest/Response
- [x] Define proto messages: GetReceiptRequest/Response, ListReceiptsRequest/Response
- [x] Implement RecordPayment handler with ledger integration
- [x] Implement GetReceipt handler
- [x] Implement ListReceipts handler with filters
- [x] Implement receipt number generation
- [x] Implement invoice status update on payment
- [x] Handle partial payments

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

**Status:** ✅ Implemented

### GetReceipt
**Input:** tenant_id, receipt_id
**Output:** receipt

**Status:** ✅ Implemented

### ListReceipts
**Input:** tenant_id, invoice_id (optional), date_from (optional), date_to (optional), page_size, page_token
**Output:** receipts[], next_page_token

**Status:** ✅ Implemented

## Payment Methods

Supported values (stored as string, not enum):
- cash
- card
- bank_transfer
- check
- upi
- other

## Acceptance Criteria

- [x] RecordPayment creates receipt with generated number
- [x] RecordPayment creates correct ledger entry
- [x] RecordPayment updates invoice amount_paid
- [x] RecordPayment sets status to paid when fully paid
- [x] RecordPayment allows partial payments
- [x] RecordPayment rejects overpayment
- [x] RecordPayment rejects payment on draft/void invoice
- [x] GetReceipt returns receipt details
- [x] ListReceipts filters by invoice_id and date range

## Integration Tests

- [x] Record full payment marks invoice as paid
- [x] Record partial payment updates balance_due
- [x] Record multiple partial payments succeeds
- [x] Record payment exceeding balance returns INVALID_ARGUMENT
- [x] Record payment on draft invoice returns FAILED_PRECONDITION
- [x] Get receipt returns payment details
- [x] List receipts for invoice returns all payments
- [x] List receipts by date range works correctly
