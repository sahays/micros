# Story: Statement Import

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement GenAI-powered statement import with staging workflow. Upload statements (PDF, CSV, images), extract transactions via genai-service, stage for user review, and commit approved transactions.

## Tasks

- [ ] Define proto messages: BankStatement, BankTransaction, ExtractionResult
- [ ] Define proto messages: ImportStatementRequest/Response
- [ ] Define proto messages: GetStatementRequest/Response, ListStatementsRequest/Response
- [ ] Define proto messages: GetStagedTransactionsRequest/Response
- [ ] Define proto messages: UpdateStagedTransactionRequest/Response
- [ ] Define proto messages: CommitStatementRequest/Response
- [ ] Define proto messages: AbandonStatementRequest/Response
- [ ] Implement ImportStatement handler (upload + trigger extraction)
- [ ] Integrate with document-service for file storage
- [ ] Integrate with genai-service for transaction extraction
- [ ] Implement GetStagedTransactions handler
- [ ] Implement UpdateStagedTransaction handler (corrections)
- [ ] Implement CommitStatement handler
- [ ] Implement AbandonStatement handler
- [ ] Implement statement validation (balances, dates)
- [ ] Add capability checks to all methods
- [ ] Add metering for extraction operations

## gRPC Methods

### ImportStatement
**Input:** tenant_id, bank_account_id, document_id (from document-service), period_start, period_end, opening_balance (optional), closing_balance (optional)
**Output:** statement_id, status: "extracting"

**Behavior:**
1. Fetch document from document-service
2. Create statement record with status "extracting"
3. Send document to genai-service for extraction (async)
4. Return immediately with statement_id
5. On extraction complete: update status to "staged"

**Capability:** `reconciliation.statement:import`

### GetStatement
**Input:** tenant_id, statement_id
**Output:** statement with extraction status, confidence scores

**Capability:** `reconciliation.statement:read`

### ListStatements
**Input:** tenant_id, bank_account_id (optional), status (optional), page_size, page_token
**Output:** statements[], next_page_token

**Capability:** `reconciliation.statement:read`

### GetStagedTransactions
**Input:** tenant_id, statement_id
**Output:** staged_transactions[] with confidence scores per field

**Note:** Only available when status is "staged"

**Capability:** `reconciliation.statement:read`

### UpdateStagedTransaction
**Input:** tenant_id, statement_id, transaction_index, field_updates (date, description, amount, etc.)
**Output:** updated_transaction

**Note:** User corrections to GenAI extraction. Tracks corrections for learning.

**Capability:** `reconciliation.statement:edit`

### CommitStatement
**Input:** tenant_id, statement_id
**Output:** statement with status: "committed", transaction_count

**Validation:**
- Statement must be in "staged" status
- Opening balance should match previous statement's closing (warn if not)
- Calculated closing (opening + transactions) should match provided closing (warn if not)

**Business Logic:**
1. Lock staged transactions (no more edits)
2. Store as committed bank_transactions
3. Update statement status to "committed"
4. Send extraction feedback to genai-service (corrections made)
5. Ready for reconciliation

**Capability:** `reconciliation.statement:commit`

### AbandonStatement
**Input:** tenant_id, statement_id, reason (optional)
**Output:** statement with status: "abandoned"

**Note:** Discard extraction results. Can re-import with same document.

**Capability:** `reconciliation.statement:edit`

## GenAI Extraction

### Request to genai-service
```json
{
  "document_id": "uuid",
  "document_url": "secure-url-from-document-service",
  "extraction_type": "bank_statement",
  "hints": {
    "bank_name": "ICICI Bank",
    "account_number_last4": "1234",
    "expected_currency": "INR"
  }
}
```

### Response from genai-service
```json
{
  "extraction_id": "uuid",
  "confidence": 0.92,
  "opening_balance": {"value": 50000.00, "confidence": 0.98},
  "closing_balance": {"value": 48500.00, "confidence": 0.97},
  "transactions": [
    {
      "date": {"value": "2026-01-15", "confidence": 0.99},
      "description": {"value": "NEFT-ACME CORP-REF123", "confidence": 0.95},
      "reference": {"value": "REF123", "confidence": 0.90},
      "debit": {"value": 1500.00, "confidence": 0.98},
      "credit": null,
      "balance": {"value": 48500.00, "confidence": 0.96}
    }
  ],
  "warnings": ["Page 2 had low image quality"]
}
```

## Staging Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  1. UPLOAD                                                   │
│     User uploads PDF/CSV/image statement                     │
│     → ImportStatement creates record, triggers extraction    │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  2. EXTRACT (async)                                          │
│     genai-service processes document                         │
│     → Extracts transactions with confidence scores           │
│     → Status: extracting → staged                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  3. REVIEW                                                   │
│     User reviews staged transactions                         │
│     → GetStagedTransactions shows extracted data             │
│     → Low confidence fields highlighted                      │
│     → User corrects errors via UpdateStagedTransaction       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  4. COMMIT                                                   │
│     User approves extraction                                 │
│     → CommitStatement locks transactions                     │
│     → Corrections sent to genai-service for learning         │
│     → Ready for reconciliation                               │
└─────────────────────────────────────────────────────────────┘
```

## Supported Formats

GenAI extraction handles:
- **PDF**: Scanned or digital, single or multi-page
- **Images**: JPG, PNG (photos of statements)
- **CSV**: Any delimiter, any column order
- **Excel**: XLS, XLSX

No bank-specific parsers required. GenAI adapts to:
- ICICI, SBI, HDFC, Axis, Kotak, etc.
- International banks (varying formats)
- Different languages (with hints)

## Metering

Record on each operation:
```rust
record_statement_import(&tenant_id);
record_extraction_request(&tenant_id);
record_extraction_correction(&tenant_id, field_count);
record_statement_commit(&tenant_id, transaction_count);
```

## Acceptance Criteria

- [ ] ImportStatement uploads and triggers extraction
- [ ] ImportStatement integrates with document-service
- [ ] Extraction via genai-service returns structured data
- [ ] GetStagedTransactions shows extraction with confidence
- [ ] UpdateStagedTransaction allows field corrections
- [ ] CommitStatement finalizes transactions
- [ ] CommitStatement validates balance continuity
- [ ] AbandonStatement allows re-import
- [ ] Low confidence fields flagged for review
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Import PDF statement triggers extraction
- [ ] Import CSV statement triggers extraction
- [ ] Get staged transactions after extraction completes
- [ ] Update staged transaction records correction
- [ ] Commit statement with valid data succeeds
- [ ] Commit statement with balance mismatch warns
- [ ] Abandon statement allows re-import
- [ ] Import overlapping period returns ALREADY_EXISTS
- [ ] Operations without capability return PERMISSION_DENIED
