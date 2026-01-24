# Story: Reconciliation Process

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement StartReconciliation, GetReconciliation, ListReconciliations, CompleteReconciliation, and AbandonReconciliation gRPC methods for managing reconciliation sessions.

## Tasks

- [ ] Define proto messages: Reconciliation, StartReconciliationRequest/Response
- [ ] Define proto messages: GetReconciliationRequest/Response
- [ ] Define proto messages: ListReconciliationsRequest/Response
- [ ] Define proto messages: CompleteReconciliationRequest/Response
- [ ] Define proto messages: AbandonReconciliationRequest/Response
- [ ] Implement StartReconciliation handler
- [ ] Implement GetReconciliation handler with status
- [ ] Implement ListReconciliations handler with filters
- [ ] Implement CompleteReconciliation handler with validation
- [ ] Implement AbandonReconciliation handler
- [ ] Integrate ledger-service client for balance queries
- [ ] Implement period locking on completion
- [ ] Add capability checks to all methods

## gRPC Methods

### StartReconciliation
**Input:** tenant_id (from auth), bank_account_id, period_start, period_end
**Output:** reconciliation

**Behavior:**
1. Validate no overlapping reconciliation in progress for this account
2. Fetch committed statements for period
3. Query ledger balance for bank's ledger_account_id as of period_end
4. Calculate actual balance from statement closing balance
5. Create reconciliation record with difference
6. Return reconciliation with initial match summary

**Capability:** `reconciliation.reconciliation:create`

### GetReconciliation
**Input:** tenant_id (from auth), reconciliation_id
**Output:** reconciliation with detailed status

**Status includes:**
- Total bank transactions in period
- Matched count
- Unmatched count
- Excluded count
- Expected vs actual balance
- Difference amount
- Status (in_progress, completed, abandoned)

**Capability:** `reconciliation.reconciliation:read`

### ListReconciliations
**Input:** tenant_id (from auth), bank_account_id, status_filter (optional), page_size, page_token
**Output:** reconciliations[], next_page_token

**Sorting:** Most recent first (by started_utc descending)

**Capability:** `reconciliation.reconciliation:read`

### CompleteReconciliation
**Input:** tenant_id (from auth), reconciliation_id
**Output:** reconciliation

**Validation:**
- All transactions must be matched or excluded
- Difference should be zero (or adjustments must cover it)
- If difference exists and no adjustments, return FAILED_PRECONDITION

**Behavior:**
1. Mark reconciliation as completed
2. Set completed_utc timestamp
3. Update bank account's last_reconciled_date and last_reconciled_balance
4. Lock period (prevent changes to matched ledger entries)

**Capability:** `reconciliation.reconciliation:complete`

### AbandonReconciliation
**Input:** tenant_id (from auth), reconciliation_id
**Output:** success

**Behavior:**
- Mark reconciliation as abandoned
- Do not lock period
- Matches remain but period not finalized
- Can start new reconciliation for same period

**Capability:** `reconciliation.reconciliation:abandon`

## Balance Verification

**Expected balance** (from ledger):
- Query ledger-service for account balance as of period_end
- Uses GetAccountBalance gRPC call

**Actual balance** (from statement):
- Closing balance from the latest committed bank statement in period

**Difference:**
- expected - actual
- Should be zero when properly reconciled
- Non-zero indicates:
  - Missing ledger entries
  - Unrecorded bank fees/interest
  - Timing differences
  - Errors

## Period Locking

Per spec: "Reconciliation locks period - no ledger changes allowed in locked periods"

When a reconciliation is completed:
1. Record the locked period (period_start to period_end) for the bank account
2. Notify ledger-service of the lock (or check on ledger entry creation)
3. Subsequent ledger entries affecting the bank's ledger account in this period should be rejected

## Business Rules

From spec:
- "Completed reconciliations are immutable; corrections require new entries"
- Cannot modify a completed reconciliation
- To fix errors, create new adjustment entries in a new period

## Acceptance Criteria

- [ ] StartReconciliation creates session for valid period
- [ ] StartReconciliation calculates correct expected balance from ledger
- [ ] StartReconciliation rejects overlapping in-progress reconciliation
- [ ] GetReconciliation returns match statistics
- [ ] ListReconciliations returns reconciliations for bank account
- [ ] ListReconciliations supports status filter
- [ ] CompleteReconciliation validates all transactions addressed
- [ ] CompleteReconciliation updates bank account last_reconciled fields
- [ ] CompleteReconciliation locks the period
- [ ] CompleteReconciliation rejects if unmatched transactions exist
- [ ] AbandonReconciliation marks incomplete without locking
- [ ] Cannot modify completed reconciliation
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Start reconciliation for valid period succeeds
- [ ] Start overlapping reconciliation returns ALREADY_EXISTS
- [ ] Get reconciliation returns accurate counts
- [ ] List reconciliations returns correct results
- [ ] List with status filter works correctly
- [ ] Complete with unmatched transactions returns FAILED_PRECONDITION
- [ ] Complete with non-zero difference and no adjustments returns FAILED_PRECONDITION
- [ ] Complete successful reconciliation updates bank account
- [ ] Complete reconciliation locks period
- [ ] Abandon reconciliation allows new reconciliation for same period
- [ ] Operations without capability return PERMISSION_DENIED
