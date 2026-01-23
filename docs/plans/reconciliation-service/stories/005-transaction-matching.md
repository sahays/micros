# Story: Transaction Matching

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement MatchTransaction, UnmatchTransaction, and ExcludeTransaction gRPC methods for manual transaction matching.

## Tasks

- [ ] Define proto messages: TransactionMatch, MatchTransactionRequest/Response
- [ ] Define proto messages: UnmatchTransactionRequest/Response, ExcludeTransactionRequest/Response
- [ ] Implement MatchTransaction handler (one-to-one)
- [ ] Implement split matching (one-to-many)
- [ ] Implement UnmatchTransaction handler
- [ ] Implement ExcludeTransaction handler
- [ ] Query ledger-service for candidate entries

## gRPC Methods

### MatchTransaction
**Input:** tenant_id, bank_transaction_id, ledger_entry_ids[] (one or more for split)
**Output:** matches[]

**Validation:**
- Bank transaction exists and is unmatched
- All ledger entries exist
- For split: sum of ledger amounts equals bank amount
- Ledger entries not already matched

### UnmatchTransaction
**Input:** tenant_id, bank_transaction_id
**Output:** success

**Behavior:**
- Remove all matches for transaction
- Set transaction status back to unmatched

### ExcludeTransaction
**Input:** tenant_id, bank_transaction_id, reason
**Output:** success

**Behavior:**
- Mark transaction as excluded
- Excluded transactions don't need matching
- Still count toward balance verification
- Use for: bank fees already recorded, internal transfers, etc.

## Split Matching

For bank transactions that correspond to multiple ledger entries:

Example: Bank deposit of $1,500
- Invoice payment: $1,000
- Invoice payment: $500
- Match bank transaction to both ledger entries

Validation:
- Sum of ledger entry amounts must equal bank transaction amount
- All entries must be same direction (all debits or all credits)

## Candidate Query

When user wants to match, provide candidates from ledger:
1. Query ledger entries for bank's ledger_account_id
2. Filter by date range (bank_date ± 7 days)
3. Filter by amount (exact or close matches)
4. Exclude already matched entries
5. Return sorted by match likelihood

## Acceptance Criteria

- [ ] MatchTransaction creates match record
- [ ] MatchTransaction supports split matching
- [ ] MatchTransaction validates amounts for splits
- [ ] UnmatchTransaction removes match
- [ ] ExcludeTransaction marks as excluded
- [ ] Excluded transactions don't appear in unmatched list
- [ ] Cannot match already matched transaction

## Integration Tests

- [ ] Match single transaction succeeds
- [ ] Match split transaction succeeds
- [ ] Match split with wrong total returns INVALID_ARGUMENT
- [ ] Unmatch transaction succeeds
- [ ] Exclude transaction succeeds
- [ ] Re-match excluded transaction fails
