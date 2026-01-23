# Story: Bank Accounts

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement RegisterBankAccount, GetBankAccount, and ListBankAccounts gRPC methods for managing bank account configurations.

## Tasks

- [ ] Define proto messages: BankAccount, RegisterBankAccountRequest/Response
- [ ] Define proto messages: GetBankAccountRequest/Response, ListBankAccountsRequest/Response
- [ ] Implement RegisterBankAccount handler
- [ ] Implement GetBankAccount handler
- [ ] Implement ListBankAccounts handler
- [ ] Validate ledger account exists via ledger-service

## gRPC Methods

### RegisterBankAccount
**Input:** tenant_id, ledger_account_id, bank_name, account_number_masked, currency, statement_format, format_config
**Output:** bank_account

**Validation:**
- ledger_account_id exists and is asset type (cash/bank)
- currency matches ledger account currency
- statement_format is supported (csv, ofx, mt940)
- format_config valid for chosen format

### GetBankAccount
**Input:** tenant_id, bank_account_id
**Output:** bank_account with reconciliation status

### ListBankAccounts
**Input:** tenant_id, page_size, page_token
**Output:** bank_accounts[], next_page_token

## Statement Formats

### CSV
format_config:
- date_column, date_format
- description_column
- amount_column (or debit_column + credit_column)
- reference_column (optional)
- balance_column (optional)
- skip_rows, delimiter

### OFX (Open Financial Exchange)
format_config:
- Standard OFX parsing, minimal config needed

### MT940 (SWIFT)
format_config:
- Standard MT940 parsing, minimal config needed

## Acceptance Criteria

- [ ] RegisterBankAccount creates account with valid config
- [ ] RegisterBankAccount validates ledger account exists
- [ ] RegisterBankAccount rejects invalid format config
- [ ] GetBankAccount returns account with last reconciled info
- [ ] ListBankAccounts returns tenant's accounts
- [ ] Account linked to correct ledger account

## Integration Tests

- [ ] Register bank account with valid data succeeds
- [ ] Register with invalid ledger account returns NOT_FOUND
- [ ] Register with mismatched currency returns INVALID_ARGUMENT
- [ ] Get bank account returns complete details
- [ ] List bank accounts returns only tenant's accounts
