# Story: Transaction Matching

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement MatchTransaction, UnmatchTransaction, ExcludeTransaction, and GetCandidateEntries gRPC methods for manual transaction matching.

## Tasks

- [ ] Define proto messages: TransactionMatch, MatchTransactionRequest/Response
- [ ] Define proto messages: UnmatchTransactionRequest/Response, ExcludeTransactionRequest/Response
- [ ] Define proto messages: GetCandidateEntriesRequest/Response
- [ ] Implement MatchTransaction handler (one-to-one)
- [ ] Implement split matching (one-to-many)
- [ ] Implement UnmatchTransaction handler
- [ ] Implement ExcludeTransaction handler
- [ ] Implement GetCandidateEntries handler
- [ ] Integrate ledger-service client for candidate queries
- [ ] Add capability checks to all methods

## gRPC Methods

### MatchTransaction
**Input:** tenant_id (from auth), bank_transaction_id, ledger_entry_ids[] (one or more for split)
**Output:** matches[]

**Validation:**
- Bank transaction exists and is unmatched
- All ledger entries exist (via ledger-service)
- For split: sum of ledger amounts equals bank amount (Business Rule #6)
- Ledger entries not already matched

**Capability:** `reconciliation.transaction:match`

### UnmatchTransaction
**Input:** tenant_id (from auth), bank_transaction_id
**Output:** success

**Behavior:**
- Remove all matches for transaction
- Set transaction status back to unmatched

**Capability:** `reconciliation.transaction:match`

### ExcludeTransaction
**Input:** tenant_id (from auth), bank_transaction_id, reason
**Output:** success

**Behavior:**
- Mark transaction as excluded
- Excluded transactions don't need matching
- Still count toward balance verification (Business Rule #5)
- Use for: bank fees already recorded, internal transfers, etc.

**Capability:** `reconciliation.transaction:exclude`

### GetCandidateEntries
**Input:** tenant_id (from auth), bank_transaction_id, date_range_days (optional, default 7)
**Output:** candidate_entries[] with match_likelihood

**Behavior:**
1. Get bank transaction details (date, amount)
2. Query ledger-service for entries on bank's ledger_account_id
3. Filter by date range (bank_date ± date_range_days)
4. Filter by amount (exact matches first, then close matches)
5. Exclude already matched entries
6. Return sorted by match likelihood (exact amount+date first)

**Note:** This helps users find the right ledger entry to match against.

**Capability:** `reconciliation.transaction:read`

## Split Matching

For bank transactions that correspond to multiple ledger entries:

**Example:** Bank deposit of $1,500
- Invoice payment: $1,000
- Invoice payment: $500
- Match bank transaction to both ledger entries

**Validation (Business Rule #6):**
- Sum of ledger entry amounts must equal bank transaction amount exactly
- All entries must be same direction (all debits or all credits)

## Candidate Entry Response

```protobuf
message CandidateEntry {
  string ledger_entry_id = 1;
  string date = 2;
  string description = 3;
  string amount = 4;
  string account_name = 5;
  double match_likelihood = 6;  // 0-1, based on amount/date proximity
}

message GetCandidateEntriesResponse {
  repeated CandidateEntry candidates = 1;
}
```

## Business Rules Enforced

- **Rule #5**: Excluded transactions still count toward balance verification
- **Rule #6**: Split matches must sum to exact bank transaction amount

## Acceptance Criteria

- [ ] MatchTransaction creates match record
- [ ] MatchTransaction supports split matching (multiple ledger entries)
- [ ] MatchTransaction validates amounts sum for splits
- [ ] MatchTransaction updates transaction status to matched
- [ ] UnmatchTransaction removes match and resets status
- [ ] ExcludeTransaction marks as excluded with reason
- [ ] Excluded transactions still count toward balance
- [ ] GetCandidateEntries returns sorted candidates from ledger
- [ ] GetCandidateEntries excludes already-matched entries
- [ ] Cannot match already matched transaction
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Match single transaction succeeds
- [ ] Match updates transaction status
- [ ] Match split transaction succeeds
- [ ] Match split with wrong total returns INVALID_ARGUMENT
- [ ] Unmatch transaction succeeds
- [ ] Unmatch resets status to unmatched
- [ ] Exclude transaction succeeds
- [ ] Exclude stores reason
- [ ] Get candidates returns sorted list
- [ ] Get candidates excludes matched entries
- [ ] Re-match excluded transaction fails
- [ ] Operations without capability return PERMISSION_DENIED
