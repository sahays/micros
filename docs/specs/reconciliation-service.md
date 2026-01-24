# Reconciliation Service

## Purpose

Parse bank statements into structured data, match transactions to ledger entries, and identify discrepancies. Uses GenAI for statement parsing (converting PDF/CSV/images to structured JSON), then rule-based and manual matching to reconcile transactions.

## Architecture Overview

```
PARSING (GenAI)          MATCHING (Rules + Manual)       RECONCILIATION
┌──────────────┐         ┌──────────────────────┐        ┌──────────────┐
│ Bank         │         │ 1. Auto-match (rules)│        │ Compare      │
│ Statement    │ ──────► │ 2. Manual matching   │ ─────► │ balances,    │
│ (PDF/CSV)    │  GenAI  │ 3. AI hints (optional)│        │ adjustments  │
└──────────────┘ parses  └──────────────────────┘        └──────────────┘
                to JSON
```

## Domain

### Bank Account
A bank account registered for reconciliation.

- Links to corresponding ledger account (cash/bank asset account)
- Contains bank name, account number (masked), currency
- Stores last reconciled date and balance

### Bank Statement
An imported statement file from a bank.

- Source file reference (uploaded via document-service)
- Statement period (start/end dates)
- Opening and closing balances as reported by bank
- Status: uploaded, extracting, staged, committed, reconciling, reconciled, failed
- Contains extracted transactions
- Extraction confidence score (from GenAI parsing)

**Status Flow:**
```
uploaded → extracting → staged → committed → reconciling → reconciled
                ↓           ↓
              failed     abandoned
```

### Bank Transaction
A single transaction parsed from a bank statement.

- Date, description, reference number
- Amount (positive for deposits, negative for withdrawals)
- Running balance (if provided)
- Status: staged, unmatched, matched, manually_matched, excluded

### Match
A link between a bank transaction and ledger entry(ies).

- Can be one-to-one or one-to-many (split transactions)
- Match type: auto (rule engine), manual (user), ai_confirmed (optional)
- User controls all matching decisions

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
- Set up matching rules for common transaction patterns

**Statement Import (GenAI Parsing)**
- Upload statement file (PDF, CSV, image) via document-service
- Send to genai-service for parsing to structured JSON
- GenAI extracts transactions regardless of bank format
- Stage parsed data for user review
- User reviews, corrects, and commits transactions
- Validate statement continuity (closing = next opening)

**Transaction Matching (Rules + Manual)**
- Auto-match using matching rules (primary)
- Manual matching by user (primary)
- Split matching (one bank txn to multiple ledger entries)
- Exclude transactions (bank fees already recorded, etc.)
- Optional: Request AI suggestions for difficult matches

**Reconciliation Process**
- Start reconciliation for account and period
- Review auto-matched transactions
- Manually match remaining transactions
- Identify discrepancies
- Create adjustments for differences
- Complete reconciliation and lock period

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

**Primary Role: Statement Parsing**

GenAI's purpose is to parse bank statements into structured JSON:

- Send statement document (PDF, image, CSV) to genai-service
- GenAI extracts structured transaction data:
  - Date, description, reference number
  - Debit/credit amount, running balance
  - Opening and closing balances
- Works with any bank format (ICICI, SBI, HDFC, Axis, etc.)
- No bank-specific parsers required
- Confidence scores per extracted field
- Handles multi-page statements, tables, varying layouts

**GenAI does NOT:**
- Match transactions to ledger entries
- Decide if transactions are matched/unmatched
- Make reconciliation decisions

**Staging Workflow:**
- Parsed transactions staged for review (not immediately committed)
- User reviews parsing accuracy
- User can correct individual fields
- User commits approved transactions
- Corrections sent back to genai-service for learning

**Optional: Matching Suggestions**
- User can request AI hints for difficult matches
- All suggestions require user confirmation
- Not a replacement for rules or manual matching

## Matching Hierarchy

1. **Rule-based Auto-matching (Primary)**
   - Matching rules applied in priority order
   - Pattern matching on description
   - First match wins
   - Runs automatically on statement commit

2. **Manual Matching (Primary)**
   - User reviews unmatched transactions
   - Match to ledger entries
   - Exclude from matching
   - Split matching

3. **AI Suggestions (Optional, Secondary)**
   - User explicitly requests suggestions
   - AI provides hints with confidence scores
   - User must confirm or reject each suggestion
   - Never auto-applied

## Business Rules

1. Statement periods must not overlap for same account
2. Reconciliation locks period - no ledger changes allowed in locked periods
3. All matches require user action (rules are user-defined, manual is user-initiated)
4. Matching rules are evaluated in priority order; first match wins
5. Excluded transactions still count toward balance verification
6. Split matches must sum to exact bank transaction amount
7. Completed reconciliations are immutable; corrections require new entries
8. Statement parsing failures mark individual transactions, not entire statement

## Dependencies

- **ledger-service**: Query balances and entries, create adjustments
- **document-service**: Store and retrieve statement files
- **genai-service**: Parse bank statements to structured JSON
