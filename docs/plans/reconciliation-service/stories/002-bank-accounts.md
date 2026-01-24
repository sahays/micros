# Story: Bank Accounts

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement RegisterBankAccount, GetBankAccount, ListBankAccounts, and UpdateBankAccount gRPC methods for managing bank account configurations.

Bank accounts link to ledger-service cash/bank accounts and serve as the anchor for statement imports and reconciliation. No bank-specific format configuration is needed - GenAI handles all statement formats automatically.

## Tasks

- [ ] Define proto messages: BankAccount, RegisterBankAccountRequest/Response
- [ ] Define proto messages: GetBankAccountRequest/Response, ListBankAccountsRequest/Response
- [ ] Define proto messages: UpdateBankAccountRequest/Response
- [ ] Implement RegisterBankAccount handler
- [ ] Implement GetBankAccount handler
- [ ] Implement ListBankAccounts handler
- [ ] Implement UpdateBankAccount handler
- [ ] Integrate ledger-service client for account validation
- [ ] Add capability checks to all methods

## gRPC Methods

### RegisterBankAccount
**Input:** tenant_id (from auth), ledger_account_id, bank_name, account_number_masked, currency
**Output:** bank_account

**Validation:**
- ledger_account_id exists and is asset type (cash/bank) via ledger-service
- currency matches ledger account currency
- account_number_masked is valid format (typically last 4 digits)

**Capability:** `reconciliation.bank_account:create`

### GetBankAccount
**Input:** tenant_id (from auth), bank_account_id
**Output:** bank_account with last_reconciled_date and last_reconciled_balance

**Capability:** `reconciliation.bank_account:read`

### ListBankAccounts
**Input:** tenant_id (from auth), page_size, page_token
**Output:** bank_accounts[], next_page_token

**Capability:** `reconciliation.bank_account:read`

### UpdateBankAccount
**Input:** tenant_id (from auth), bank_account_id, bank_name (optional), account_number_masked (optional)
**Output:** bank_account

**Note:** ledger_account_id and currency cannot be changed after creation.

**Capability:** `reconciliation.bank_account:update`

## GenAI-First Approach

Per the spec, statement format detection is handled entirely by GenAI:

> "GenAI handles format detection automatically"
> "No bank-specific parsers required"

This means:
- No `statement_format` or `format_config` fields on BankAccount
- Any bank format (ICICI, SBI, HDFC, Axis, international) works automatically
- PDF, CSV, images, Excel - all handled by genai-service
- Optional extraction hints can be provided at import time (Story 003)

## Acceptance Criteria

- [ ] RegisterBankAccount creates account with valid data
- [ ] RegisterBankAccount validates ledger account exists via ledger-service
- [ ] RegisterBankAccount validates currency matches ledger account
- [ ] GetBankAccount returns account with last reconciled info
- [ ] ListBankAccounts returns only tenant's accounts with pagination
- [ ] UpdateBankAccount updates mutable fields only
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Register bank account with valid data succeeds
- [ ] Register with invalid ledger account returns NOT_FOUND
- [ ] Register with mismatched currency returns INVALID_ARGUMENT
- [ ] Register with duplicate ledger_account_id returns ALREADY_EXISTS
- [ ] Get bank account returns complete details
- [ ] Get non-existent account returns NOT_FOUND
- [ ] List bank accounts returns only tenant's accounts
- [ ] List with pagination works correctly
- [ ] Update bank account modifies allowed fields
- [ ] Update cannot change ledger_account_id or currency
- [ ] Operations without capability return PERMISSION_DENIED
