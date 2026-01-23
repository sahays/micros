# Story: Idempotency

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement idempotency keys to prevent duplicate transactions and handle concurrent requests.

## Tasks

- [ ] Add unique constraint on idempotency_key
- [ ] Check for existing transaction before insert
- [ ] Return existing transaction if idempotency_key matches
- [ ] Handle race conditions with database constraint
- [ ] Add idempotency_key to response for client tracking

## Idempotency Flow

1. Client sends PostTransaction with idempotency_key
2. Check if idempotency_key exists in ledger_entries
3. If exists → return original transaction (journal_id, entries)
4. If not exists → insert new transaction
5. On constraint violation → retry lookup and return existing

## Concurrency Handling

- Use UNIQUE constraint on idempotency_key
- Catch unique violation error
- Retry lookup on violation
- Return existing transaction

## Acceptance Criteria

- [ ] First request with idempotency_key creates transaction
- [ ] Second request with same key returns same transaction
- [ ] Different key creates new transaction
- [ ] Concurrent requests with same key result in single transaction
- [ ] Response includes idempotency_key used

## Integration Tests

- [ ] Duplicate request returns original journal_id
- [ ] Duplicate request does not create additional entries
- [ ] Different idempotency_key creates separate transaction
- [ ] Null idempotency_key allows duplicate transactions
