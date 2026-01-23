# Story: Statements

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement GenerateStatement gRPC method for on-demand customer account statements.

## Design Decision

**Statements are ephemeral** - generated on-demand, not stored. This simplifies implementation:
- No statement storage table needed
- No cache invalidation complexity
- Always reflects current data
- PDF can be generated separately via GenerateStatementPdf

## Tasks

- [x] Define proto messages: Statement, StatementLine, GenerateStatementRequest/Response
- [x] Implement GenerateStatement handler
- [x] Implement opening balance calculation (pre-period invoice/payment aggregation)
- [x] Implement statement line aggregation (invoices and receipts in period)
- [x] Calculate running balance for each line

## gRPC Method

### GenerateStatement
**Input:** tenant_id, customer_id, period_start, period_end
**Output:** statement

**Algorithm:**
1. Calculate opening balance:
   - Sum all issued invoices before period_start
   - Subtract all payments before period_start
2. Fetch invoices issued within [period_start, period_end]
3. Fetch receipts recorded within [period_start, period_end]
4. Merge and sort by date
5. Calculate running balance for each line
6. Return statement with all fields populated

**Status:** ✅ Implemented

## Statement Structure

**Statement:**
- tenant_id
- customer_id
- customer_name
- billing_address
- currency
- period_start
- period_end
- opening_balance
- closing_balance
- total_debits (sum of invoices in period)
- total_credits (sum of payments/credit notes in period)
- lines[]
- generated_at

**StatementLine:**
- date
- document_type (invoice, credit_note, payment)
- document_number
- description
- debit (invoice amounts)
- credit (payment/credit note amounts)
- balance (running balance)

## Acceptance Criteria

- [x] GenerateStatement calculates correct opening balance
- [x] GenerateStatement includes all issued invoices in period
- [x] GenerateStatement includes all payments in period
- [x] GenerateStatement calculates correct closing balance
- [x] Statement lines ordered by date ascending
- [x] Running balance calculated correctly for each line
- [x] Credit notes appear as credits (negative effect on balance)
- [x] Statements are tenant and customer isolated
- [x] Invalid date range returns INVALID_ARGUMENT
- [x] Invalid customer ID returns INVALID_ARGUMENT

## Integration Tests

- [x] Generate statement for customer with invoices and payments
- [x] Generate statement for customer with no activity in period
- [x] Opening balance correctly reflects prior period activity
- [x] Statement lines sorted chronologically
- [x] Closing balance = opening + debits - credits
- [x] Running balance tracked correctly
- [x] Invalid date range returns error
- [x] Invalid customer ID returns error
