# Story: Account Management

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Implement CreateAccount, GetAccount, and ListAccounts gRPC methods.

## Tasks

- [ ] Define proto messages: Account, CreateAccountRequest/Response
- [ ] Define proto messages: GetAccountRequest/Response, ListAccountsRequest/Response
- [ ] Implement CreateAccount handler with validation
- [ ] Implement GetAccount handler with balance calculation
- [ ] Implement ListAccounts handler with filters and pagination
- [ ] Add tenant context extraction from gRPC metadata
- [ ] Write database queries with sqlx

## gRPC Methods

### CreateAccount
**Input:** tenant_id, account_type, account_code, currency, allow_negative, metadata
**Output:** account

**Validation:**
- account_type in (asset, liability, equity, revenue, expense)
- currency is valid ISO 4217
- account_code unique within tenant

### GetAccount
**Input:** tenant_id, account_id
**Output:** account with current balance

**Balance:** Sum of debits - sum of credits (for asset/expense accounts)

### ListAccounts
**Input:** tenant_id, account_type (optional), currency (optional), page_size, page_token
**Output:** accounts[], next_page_token

## Acceptance Criteria

- [ ] CreateAccount returns new account with generated ID
- [ ] CreateAccount rejects duplicate account_code within tenant
- [ ] CreateAccount rejects invalid account_type or currency
- [ ] GetAccount returns account with calculated balance
- [ ] GetAccount returns NOT_FOUND for missing account
- [ ] ListAccounts filters by account_type and currency
- [ ] ListAccounts pagination works correctly
- [ ] All methods enforce tenant isolation

## Integration Tests

- [ ] Create account with valid data returns account
- [ ] Create duplicate account_code returns ALREADY_EXISTS
- [ ] Get account returns zero balance for new account
- [ ] List accounts returns only tenant's accounts
- [ ] List accounts with filters returns matching subset
