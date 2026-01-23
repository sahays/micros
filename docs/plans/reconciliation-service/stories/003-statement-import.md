# Story: Statement Import

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement ImportStatement, GetStatement, and ListStatements gRPC methods for uploading and parsing bank statements.

## Tasks

- [ ] Define proto messages: BankStatement, BankTransaction
- [ ] Define proto messages: ImportStatementRequest/Response
- [ ] Define proto messages: GetStatementRequest/Response, ListStatementsRequest/Response
- [ ] Implement CSV parser with configurable columns
- [ ] Implement OFX parser
- [ ] Implement MT940 parser (future)
- [ ] Implement ImportStatement handler
- [ ] Integrate with document-service for file storage
- [ ] Implement statement validation (balances, dates)

## gRPC Methods

### ImportStatement
**Input:** tenant_id, bank_account_id, document_id (from document-service), period_start, period_end, opening_balance, closing_balance
**Output:** statement with parsed transactions

**Behavior:**
1. Fetch document from document-service
2. Parse based on bank account's format config
3. Extract transactions with dates, descriptions, amounts
4. Validate: closing = opening + sum(transactions)
5. Store statement and transactions
6. Return parsed result

### GetStatement
**Input:** tenant_id, statement_id
**Output:** statement with transactions and match status

### ListStatements
**Input:** tenant_id, bank_account_id (optional), status (optional), page_size, page_token
**Output:** statements[], next_page_token

## Parsing Logic

### CSV Parsing
1. Read file with configured delimiter
2. Skip header rows as configured
3. For each row:
   - Extract date using date_column and date_format
   - Extract description from description_column
   - Extract amount (single column or debit/credit columns)
   - Extract reference if configured
4. Sort by date

### Validation
- Statement period must not overlap with existing statements
- Opening balance should match previous statement's closing
- Calculated closing (opening + transactions) should match provided closing
- Flag discrepancies but don't fail import

## Acceptance Criteria

- [ ] ImportStatement parses CSV correctly
- [ ] ImportStatement parses OFX correctly
- [ ] ImportStatement extracts all transactions
- [ ] ImportStatement validates balance continuity
- [ ] ImportStatement flags parse errors per transaction
- [ ] GetStatement returns transactions with match status
- [ ] ListStatements filters by account and status
- [ ] Overlapping periods rejected

## Integration Tests

- [ ] Import valid CSV statement succeeds
- [ ] Import statement with balance mismatch warns
- [ ] Import overlapping period returns ALREADY_EXISTS
- [ ] Get statement returns all transactions
- [ ] Malformed rows flagged but import continues
