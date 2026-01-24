# Story: Statement Import

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement GenAI-powered statement parsing with staging workflow. Upload statements (PDF, CSV, images), **parse to structured JSON** via genai-service, stage for user review, and commit approved transactions.

**Key Concept**: GenAI's role is **extraction/parsing only** - converting unstructured bank statements into structured transaction data. Matching transactions to ledger entries happens separately via matching rules and manual user actions (Stories 004, 005).

## Tasks

- [ ] Define proto messages: BankStatement, BankTransaction
- [ ] Define proto messages: ImportStatementRequest/Response
- [ ] Define proto messages: GetStatementRequest/Response, ListStatementsRequest/Response
- [ ] Define proto messages: GetStagedTransactionsRequest/Response
- [ ] Define proto messages: UpdateStagedTransactionRequest/Response
- [ ] Define proto messages: CommitStatementRequest/Response
- [ ] Define proto messages: AbandonStatementRequest/Response
- [ ] Implement ImportStatement handler (upload + trigger extraction)
- [ ] Integrate with document-service for file retrieval
- [ ] Integrate with genai-service for statement parsing
- [ ] Define structured JSON schema for extraction output
- [ ] Implement GetStagedTransactions handler
- [ ] Implement UpdateStagedTransaction handler (corrections)
- [ ] Implement CommitStatement handler
- [ ] Implement AbandonStatement handler
- [ ] Implement statement validation (balances, dates)
- [ ] Add capability checks to all methods
- [ ] Add metering for extraction operations

## GenAI Role: Parsing Only

GenAI's **sole purpose** in reconciliation-service is to parse bank statements into structured data:

```
┌─────────────────────┐         ┌─────────────────────┐
│  Unstructured       │         │  Structured JSON    │
│  Bank Statement     │  ────►  │  Transaction Data   │
│  (PDF/CSV/Image)    │  GenAI  │  (Schema below)     │
└─────────────────────┘         └─────────────────────┘
                                         │
                                         ▼
                        ┌─────────────────────────────┐
                        │  Matching (Stories 004-005) │
                        │  • Rule-based auto-match    │
                        │  • Manual user matching     │
                        └─────────────────────────────┘
```

**GenAI does NOT**:
- Match transactions to ledger entries
- Decide if transactions are matched/unmatched
- Make reconciliation decisions

**GenAI DOES**:
- Parse any bank statement format (PDF, CSV, images)
- Extract transaction data to structured schema
- Provide confidence scores for extracted fields
- Learn from user corrections to improve future parsing

## Structured Output Schema

GenAI extracts to this JSON schema:

```json
{
  "statement": {
    "period_start": "2026-01-01",
    "period_end": "2026-01-31",
    "opening_balance": {
      "value": "50000.00",
      "confidence": 0.98
    },
    "closing_balance": {
      "value": "48500.00",
      "confidence": 0.97
    }
  },
  "transactions": [
    {
      "date": {
        "value": "2026-01-15",
        "confidence": 0.99
      },
      "description": {
        "value": "NEFT-ACME CORP-REF123",
        "confidence": 0.95
      },
      "reference": {
        "value": "REF123",
        "confidence": 0.90
      },
      "amount": {
        "value": "-1500.00",
        "confidence": 0.98
      },
      "running_balance": {
        "value": "48500.00",
        "confidence": 0.96
      }
    }
  ],
  "extraction_metadata": {
    "overall_confidence": 0.92,
    "page_count": 3,
    "warnings": ["Page 2 had low image quality"]
  }
}
```

This structured data then feeds into:
- **Auto-matching** (Story 004): Rules match on `description` patterns
- **Manual matching** (Story 005): User sees transactions and matches to ledger

## gRPC Methods

### ImportStatement
**Input:** tenant_id (from auth), bank_account_id, document_id (from document-service), extraction_hints (optional)
**Output:** statement_id, status: "extracting"

**Behavior:**
1. Validate bank_account_id exists
2. Fetch document metadata from document-service
3. Create statement record with status "extracting"
4. Send document to genai-service for parsing (async)
5. Return immediately with statement_id
6. On parsing complete: update status to "staged", store extracted transactions

**Capability:** `reconciliation.statement:import`

### GetStatement
**Input:** tenant_id (from auth), statement_id
**Output:** statement with extraction status, confidence scores

**Capability:** `reconciliation.statement:read`

### ListStatements
**Input:** tenant_id (from auth), bank_account_id (optional), status (optional), page_size, page_token
**Output:** statements[], next_page_token

**Capability:** `reconciliation.statement:read`

### GetStagedTransactions
**Input:** tenant_id (from auth), statement_id
**Output:** staged_transactions[] with confidence scores per field

**Note:** Only available when status is "staged". Returns parsed transactions for review.

**Capability:** `reconciliation.statement:read`

### UpdateStagedTransaction
**Input:** tenant_id (from auth), transaction_id, field_updates (date, description, amount, etc.)
**Output:** updated_transaction

**Note:** User corrections to GenAI parsing. Tracks corrections for learning feedback.

**Capability:** `reconciliation.statement:edit`

### CommitStatement
**Input:** tenant_id (from auth), statement_id
**Output:** statement with status: "committed", transaction_count

**Validation:**
- Statement must be in "staged" status
- Opening balance should match previous statement's closing (warn if not)
- Calculated closing (opening + sum of transactions) should match extracted closing (warn if not)

**Business Logic:**
1. Lock staged transactions (no more edits)
2. Change transaction status from "staged" to "unmatched"
3. Update statement status to "committed"
4. Send extraction feedback to genai-service (corrections made)
5. Transactions now available for matching (Stories 004-005)

**Capability:** `reconciliation.statement:commit`

### AbandonStatement
**Input:** tenant_id (from auth), statement_id, reason (optional)
**Output:** statement with status: "abandoned"

**Note:** Discard extraction results. Can re-import with same document.

**Capability:** `reconciliation.statement:edit`

## GenAI Parsing Request

### Request to genai-service
```json
{
  "document_id": "uuid",
  "document_url": "secure-url-from-document-service",
  "extraction_type": "bank_statement",
  "output_schema": "bank_statement_v1",
  "hints": {
    "bank_name": "ICICI Bank",
    "account_number_last4": "1234",
    "expected_currency": "INR"
  }
}
```

### Response from genai-service
Returns the structured JSON schema defined above.

## Staging Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  1. UPLOAD                                                   │
│     User uploads PDF/CSV/image statement                     │
│     → ImportStatement creates record, triggers parsing       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  2. PARSE (async via GenAI)                                  │
│     genai-service parses document to structured JSON         │
│     → Extracts transactions with confidence scores           │
│     → Status: extracting → staged                            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  3. REVIEW                                                   │
│     User reviews parsed transactions                         │
│     → GetStagedTransactions shows extracted data             │
│     → Low confidence fields highlighted                      │
│     → User corrects parsing errors via UpdateStagedTxn       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  4. COMMIT                                                   │
│     User approves parsed data                                │
│     → CommitStatement locks transactions                     │
│     → Corrections sent to genai-service for learning         │
│     → Transactions now "unmatched", ready for matching       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  5. MATCHING (Stories 004-005)                               │
│     Separate phase after commit                              │
│     → Auto-match via rules (Story 004)                       │
│     → Manual matching by user (Story 005)                    │
└─────────────────────────────────────────────────────────────┘
```

## Supported Formats

GenAI parsing handles:
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
record_genai_parsing_request(&tenant_id);
record_parsing_correction(&tenant_id, field_count);
record_statement_commit(&tenant_id, transaction_count);
```

## Acceptance Criteria

- [ ] ImportStatement triggers GenAI parsing
- [ ] ImportStatement integrates with document-service
- [ ] Parsing via genai-service returns structured JSON
- [ ] GetStagedTransactions shows parsed data with confidence
- [ ] UpdateStagedTransaction allows field corrections
- [ ] CommitStatement finalizes transactions as "unmatched"
- [ ] CommitStatement validates balance continuity
- [ ] AbandonStatement allows re-import
- [ ] Low confidence fields flagged for review
- [ ] Parsing corrections sent to genai-service for learning
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Import PDF statement triggers parsing
- [ ] Import CSV statement triggers parsing
- [ ] Import image statement triggers parsing
- [ ] Get staged transactions after parsing completes
- [ ] Update staged transaction records correction
- [ ] Commit statement changes status to "committed"
- [ ] Committed transactions have status "unmatched"
- [ ] Commit with balance mismatch warns but succeeds
- [ ] Abandon statement allows re-import
- [ ] Import overlapping period returns ALREADY_EXISTS
- [ ] Operations without capability return PERMISSION_DENIED
