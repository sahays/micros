# Story: Statements

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement GetStatement for account transaction history with opening and closing balances.

## Tasks

- [ ] Define proto messages: GetStatementRequest/Response, StatementLine
- [ ] Calculate opening balance (as of start_date - 1 day)
- [ ] Fetch entries within date range
- [ ] Calculate running balance for each entry
- [ ] Calculate closing balance

## gRPC Method

### GetStatement
**Input:** tenant_id, account_id, start_date, end_date
**Output:** account_id, currency, opening_balance, closing_balance, lines[]

**StatementLine:** entry_id, effective_date, direction, amount, running_balance, metadata

## Acceptance Criteria

- [ ] Opening balance calculated from entries before start_date
- [ ] Lines include all entries within date range
- [ ] Running balance calculated correctly for each line
- [ ] Closing balance equals opening + net change
- [ ] Lines ordered by effective_date ascending

## Integration Tests

- [ ] Statement with no prior entries has zero opening balance
- [ ] Statement includes entries within date range only
- [ ] Running balance accumulates correctly through lines
- [ ] Closing balance matches final running balance
