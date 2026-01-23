# Story: Balance Queries

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement GetBalance and GetBalances with support for point-in-time queries.

## Tasks

- [ ] Define proto messages: GetBalanceRequest/Response, GetBalancesRequest/Response
- [ ] Implement balance calculation from entries
- [ ] Implement point-in-time balance (as_of_date filter)
- [ ] Implement batch balance query for multiple accounts
- [ ] Optimize with appropriate indexes

## gRPC Methods

### GetBalance
**Input:** tenant_id, account_id, as_of_date (optional)
**Output:** account_id, balance, currency, as_of_date

**Calculation:**
- Filter entries by account_id and effective_date <= as_of_date
- Sum debits, sum credits
- Balance = debits - credits (for debit-normal accounts)
- Balance = credits - debits (for credit-normal accounts)

### GetBalances
**Input:** tenant_id, account_ids[], as_of_date (optional)
**Output:** balances[]

**Optimization:** Single query with GROUP BY account_id

## Balance Calculation

| Account Type | Formula |
|--------------|---------|
| Asset | SUM(debit) - SUM(credit) |
| Expense | SUM(debit) - SUM(credit) |
| Liability | SUM(credit) - SUM(debit) |
| Equity | SUM(credit) - SUM(debit) |
| Revenue | SUM(credit) - SUM(debit) |

## Acceptance Criteria

- [ ] GetBalance returns current balance for account
- [ ] GetBalance with as_of_date returns historical balance
- [ ] GetBalance returns zero for account with no entries
- [ ] GetBalances returns balances for all requested accounts
- [ ] GetBalances excludes accounts from other tenants
- [ ] Point-in-time excludes entries after as_of_date

## Integration Tests

- [ ] Balance after single transaction equals transaction amount
- [ ] Balance after multiple transactions equals sum
- [ ] Point-in-time balance excludes future entries
- [ ] Batch balance returns correct values for each account
- [ ] Balance of non-existent account returns NOT_FOUND
