# Story: Transaction Posting

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement PostTransaction with double-entry validation and atomic commit.

## Tasks

- [ ] Define proto messages: LedgerEntry, PostTransactionRequest/Response
- [ ] Implement double-entry validation (debits = credits)
- [ ] Implement currency consistency check
- [ ] Implement atomic insert of all entries
- [ ] Implement negative balance check (if account disallows)
- [ ] Generate journal_id to group entries
- [ ] Add effective_date and posted_utc handling

## gRPC Method

### PostTransaction
**Input:** tenant_id, entries[], idempotency_key, effective_date, metadata
**Entry:** account_id, amount, direction (debit/credit)
**Output:** journal_id, entries[]

**Validation:**
- At least 2 entries
- Sum of debit amounts = sum of credit amounts
- All accounts exist and belong to tenant
- All accounts have same currency
- Negative balance check for accounts with allow_negative=false

## Double-Entry Rules

| Account Type | Normal Balance | Increase | Decrease |
|--------------|----------------|----------|----------|
| Asset | Debit | Debit | Credit |
| Expense | Debit | Debit | Credit |
| Liability | Credit | Credit | Debit |
| Equity | Credit | Credit | Debit |
| Revenue | Credit | Credit | Debit |

## Acceptance Criteria

- [ ] Balanced transaction posts successfully
- [ ] Unbalanced transaction rejected with INVALID_ARGUMENT
- [ ] Mixed currency transaction rejected
- [ ] Non-existent account rejected with NOT_FOUND
- [ ] Cross-tenant account access rejected with PERMISSION_DENIED
- [ ] Negative balance blocked when allow_negative=false
- [ ] All entries share same journal_id
- [ ] posted_utc set to server time

## Integration Tests

- [ ] Post balanced debit/credit creates entries
- [ ] Post unbalanced transaction returns error with difference amount
- [ ] Post to closed account returns FAILED_PRECONDITION
- [ ] Post causing negative balance on restricted account returns error
- [ ] Post with backdated effective_date succeeds
