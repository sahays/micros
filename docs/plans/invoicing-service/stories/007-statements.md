# Story: Statements

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement GenerateStatement and GetStatement gRPC methods for customer account statements.

## Tasks

- [ ] Define proto messages: Statement, StatementLine, GenerateStatementRequest/Response
- [ ] Define proto messages: GetStatementRequest/Response
- [ ] Implement GenerateStatement handler
- [ ] Implement GetStatement handler
- [ ] Implement opening balance calculation
- [ ] Implement statement line aggregation
- [ ] Implement statement caching (optional)

## gRPC Methods

### GenerateStatement
**Input:** tenant_id, customer_id, period_start, period_end
**Output:** statement

**Behavior:**
1. Calculate opening balance (sum of all invoices - payments before period_start)
2. Fetch invoices issued in period
3. Fetch receipts received in period
4. Calculate closing balance
5. Store statement record
6. Return statement with lines

### GetStatement
**Input:** tenant_id, statement_id
**Output:** statement

## Statement Structure

**Statement:**
- statement_id
- tenant_id
- customer_id
- statement_date (generation date)
- period_start
- period_end
- opening_balance
- total_invoiced (sum of invoices in period)
- total_paid (sum of receipts in period)
- closing_balance (opening + invoiced - paid)
- currency
- lines[]

**StatementLine:**
- date
- type (invoice, payment)
- reference (invoice_number or receipt_number)
- description
- amount (positive for invoices, negative for payments)
- running_balance

## Acceptance Criteria

- [ ] GenerateStatement calculates correct opening balance
- [ ] GenerateStatement includes all invoices in period
- [ ] GenerateStatement includes all payments in period
- [ ] GenerateStatement calculates correct closing balance
- [ ] Statement lines ordered by date
- [ ] Statement lines show running balance
- [ ] GetStatement returns cached statement
- [ ] Statements are tenant and customer isolated

## Integration Tests

- [ ] Generate statement for customer with activity
- [ ] Generate statement for customer with no activity
- [ ] Opening balance includes prior period invoices/payments
- [ ] Statement lines sorted chronologically
- [ ] Running balance calculated correctly
