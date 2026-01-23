# Reconciliation Service

## Purpose

Parse bank statements, match transactions to ledger entries, and identify discrepancies. Uses GenAI for intelligent transaction matching and categorization.

## Domain

### Bank Account
A bank account registered for reconciliation.

- Links to corresponding ledger account (cash/bank asset account)
- Contains bank name, account number (masked), currency
- Stores last reconciled date and balance
- Supports multiple statement formats per bank

### Bank Statement
An imported statement file from a bank.

- Source file reference (uploaded via document-service)
- Statement period (start/end dates)
- Opening and closing balances as reported by bank
- Status: uploaded, extracting, staged, committed, reconciling, reconciled, failed
- Contains extracted transactions
- Extraction confidence score (from GenAI)

**Status Flow:**
```
uploaded → extracting → staged → committed → reconciling → reconciled
                ↓           ↓
              failed     abandoned
```

### Bank Transaction
A single transaction from a bank statement.

- Date, description, reference number
- Amount (positive for deposits, negative for withdrawals)
- Running balance (if provided)
- Status: unmatched, matched, manually_matched, excluded

### Match
A link between a bank transaction and ledger entry(ies).

- Can be one-to-one or one-to-many (split transactions)
- Match type: auto (system), manual (user), suggested (AI)
- Confidence score for AI-suggested matches
- User can confirm or reject suggestions

### Reconciliation
A reconciliation session for a bank account and period.

- Period being reconciled
- Expected balance (from ledger)
- Actual balance (from statement)
- Difference amount
- Status: in_progress, completed, abandoned
- Tracks matched/unmatched counts

### Matching Rule
User-defined rules for automatic matching.

- Pattern matching on transaction description
- Maps to specific ledger accounts or transaction types
- Priority ordering for rule application
- Tenant-scoped

## Key Operations

**Bank Account Setup**
- Register bank account with ledger account mapping
- Optional: Configure extraction hints (bank name, typical format patterns)
- Set up matching rules
- GenAI handles format detection automatically

**Statement Import (GenAI-Powered)**
- Upload statement file (PDF, CSV, image) via document-service
- Send to genai-service for intelligent extraction
- GenAI extracts transactions regardless of bank format
- Stage extracted data for user review
- User reviews, corrects, and commits transactions
- Validate statement continuity (closing = next opening)

**Transaction Matching**
- Auto-match using existing rules
- AI-suggest matches for unmatched transactions
- Manual matching interface
- Split matching (one bank txn to multiple ledger entries)
- Exclude transactions (bank fees already recorded, etc.)

**AI Matching (via genai-service)**
- Analyze unmatched bank transactions
- Find likely ledger entry matches based on:
  - Amount (exact or near)
  - Date proximity
  - Description similarity
  - Historical patterns
- Return confidence scores and explanations
- Learn from user confirmations/rejections

**Reconciliation Process**
- Start reconciliation for account and period
- Review auto-matched and AI-suggested matches
- Manually match remaining transactions
- Identify discrepancies
- Complete reconciliation and lock period
- Generate reconciliation report

**Discrepancy Handling**
- Flag timing differences (cleared vs booked)
- Identify missing ledger entries
- Identify duplicate ledger entries
- Create adjustment entries via ledger-service

## Ledger Integration

**Balance Verification:**
- Query ledger balance for account as of statement date
- Compare with statement closing balance

**Entry Matching:**
- Query ledger entries for account within date range
- Match entries to bank transactions by amount/date

**Adjustment Entries:**
- Create journal entries for identified discrepancies
- Record bank fees, interest, corrections

## GenAI Integration

**Statement Extraction (Primary):**
- Send statement document (PDF, image, CSV) to genai-service
- GenAI extracts structured transaction data:
  - Date, description, reference number
  - Debit/credit amount, running balance
  - Opening and closing balances
- Works with any bank format (ICICI, SBI, HDFC, Axis, etc.)
- No bank-specific parsers required
- Confidence scores per extracted field
- Handles multi-page statements, tables, varying layouts

**Staging Workflow:**
- Extracted transactions staged for review (not immediately committed)
- User reviews extraction accuracy
- User can correct individual fields
- User commits approved transactions
- Rejected extractions can be re-processed with hints

**Matching Assistance:**
- Send unmatched transactions and candidate ledger entries to genai-service
- Request match suggestions with confidence scores
- Provide historical matching patterns as context

**Description Parsing:**
- Extract payee/payer from transaction descriptions
- Identify transaction category (payment, transfer, fee, etc.)
- Normalize merchant names

**Learning:**
- Track which suggestions user accepts/rejects
- Track extraction corrections to improve future extractions
- Use feedback to improve future suggestions (per tenant)

## Business Rules

1. Statement periods must not overlap for same account
2. Reconciliation locks period - no ledger changes allowed in locked periods
3. AI suggestions require user confirmation before becoming matches
4. Matching rules are evaluated in priority order; first match wins
5. Excluded transactions still count toward balance verification
6. Split matches must sum to exact bank transaction amount
7. Completed reconciliations are immutable; corrections require new entries
8. Statement parsing failures mark individual transactions, not entire statement
9. Confidence threshold configurable per tenant for auto-accepting AI matches

## Dependencies

- **ledger-service**: Query balances and entries, create adjustments
- **document-service**: Store and retrieve statement files
- **genai-service**: AI-powered transaction matching and categorization
