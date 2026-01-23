# Story: Reconciliation Process

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement StartReconciliation, GetReconciliation, CompleteReconciliation, and AbandonReconciliation gRPC methods for managing reconciliation sessions.

## Tasks

- [ ] Define proto messages: Reconciliation, StartReconciliationRequest/Response
- [ ] Define proto messages: GetReconciliationRequest/Response
- [ ] Define proto messages: CompleteReconciliationRequest/Response, AbandonReconciliationRequest/Response
- [ ] Implement StartReconciliation handler
- [ ] Implement GetReconciliation handler with status
- [ ] Implement CompleteReconciliation handler with validation
- [ ] Implement AbandonReconciliation handler
- [ ] Query ledger-service for expected balance

## gRPC Methods

### StartReconciliation
**Input:** tenant_id, bank_account_id, period_start, period_end
**Output:** reconciliation

**Behavior:**
1. Validate no overlapping reconciliation in progress
2. Fetch statements for period
3. Query ledger balance for bank's ledger_account_id as of period_end
4. Calculate actual balance from statements
5. Create reconciliation record with difference
6. Return reconciliation with match summary

### GetReconciliation
**Input:** tenant_id, reconciliation_id
**Output:** reconciliation with detailed status

**Status includes:**
- Total bank transactions
- Matched count
- Unmatched count
- Excluded count
- Expected vs actual balance
- Difference amount

### CompleteReconciliation
**Input:** tenant_id, reconciliation_id
**Output:** reconciliation

**Validation:**
- All transactions must be matched or excluded
- Difference should be zero (or within tolerance)
- If difference exists, must have adjustment entries

**Behavior:**
- Mark reconciliation as completed
- Update bank account's last_reconciled_date and balance
- Lock period (prevent changes to matched entries)

### AbandonReconciliation
**Input:** tenant_id, reconciliation_id
**Output:** success

**Behavior:**
- Mark reconciliation as abandoned
- Do not lock period
- Matches remain but period not finalized

## Balance Verification

**Expected balance** (from ledger):
- Query ledger-service for account balance as of period_end

**Actual balance** (from statement):
- Closing balance from bank statement

**Difference:**
- expected - actual
- Should be zero when reconciled
- Non-zero indicates missing entries or errors

## Acceptance Criteria

- [ ] StartReconciliation creates session
- [ ] StartReconciliation calculates correct expected balance
- [ ] GetReconciliation returns match statistics
- [ ] CompleteReconciliation validates all matched
- [ ] CompleteReconciliation updates bank account
- [ ] CompleteReconciliation rejects with unmatched transactions
- [ ] AbandonReconciliation marks incomplete
- [ ] Cannot start overlapping reconciliation

## Integration Tests

- [ ] Start reconciliation for valid period succeeds
- [ ] Start overlapping reconciliation returns ALREADY_EXISTS
- [ ] Complete with unmatched transactions returns FAILED_PRECONDITION
- [ ] Complete successful reconciliation updates bank account
- [ ] Abandon reconciliation preserves matches
