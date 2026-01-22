# Story: Transaction Queries

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement GetTransaction and ListTransactions for retrieving ledger entries.

## Tasks

- [ ] Define proto messages: GetTransactionRequest/Response, ListTransactionsRequest/Response
- [ ] Implement GetTransaction by journal_id
- [ ] Implement ListTransactions with filters
- [ ] Add pagination support
- [ ] Add date range filtering

## gRPC Methods

### GetTransaction
**Input:** tenant_id, journal_id
**Output:** journal_id, entries[], effective_date, posted_utc, metadata

### ListTransactions
**Input:** tenant_id, account_id (optional), start_date, end_date, page_size, page_token
**Output:** transactions[], next_page_token

## Acceptance Criteria

- [ ] GetTransaction returns all entries for journal_id
- [ ] GetTransaction returns NOT_FOUND for missing journal
- [ ] ListTransactions filters by account_id
- [ ] ListTransactions filters by date range
- [ ] ListTransactions pagination works correctly
- [ ] Results ordered by effective_date descending

## Integration Tests

- [ ] Get transaction returns all entries grouped by journal
- [ ] List transactions for account returns only that account's entries
- [ ] List transactions with date range excludes outside entries
- [ ] Pagination returns correct pages with stable ordering
