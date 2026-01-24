# Epic: Reconciliation Service

Status: planning
Created: 2026-01-23

## Overview

Bank statement reconciliation service that parses bank statements and matches transactions to ledger entries. Uses GenAI for statement parsing (converting PDF/CSV/images to structured data), then rule-based and manual matching to reconcile transactions.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     RECONCILIATION WORKFLOW                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  PHASE 1: PARSING (GenAI)                                           │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐         │
│  │ Bank        │      │ GenAI       │      │ Structured  │         │
│  │ Statement   │ ───► │ Parsing     │ ───► │ JSON        │         │
│  │ (PDF/CSV)   │      │ (genai-svc) │      │ Transactions│         │
│  └─────────────┘      └─────────────┘      └─────────────┘         │
│                                                   │                 │
│                                                   ▼                 │
│  PHASE 2: MATCHING (Rules + Manual)                                 │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │                                                         │       │
│  │  1. Auto-Match (Rule Engine)     ─── PRIMARY            │       │
│  │     Pattern matching on description                     │       │
│  │     Amount/date matching to ledger                      │       │
│  │                                                         │       │
│  │  2. Manual Match (User)          ─── PRIMARY            │       │
│  │     User reviews and matches transactions               │       │
│  │     Match / Exclude / Skip                              │       │
│  │                                                         │       │
│  │  3. AI Suggestions (Optional)    ─── SECONDARY          │       │
│  │     User can request AI hints for difficult matches     │       │
│  │     Always requires user confirmation                   │       │
│  │                                                         │       │
│  └─────────────────────────────────────────────────────────┘       │
│                                                   │                 │
│                                                   ▼                 │
│  PHASE 3: RECONCILIATION                                            │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐         │
│  │ Compare     │      │ Resolve     │      │ Complete &  │         │
│  │ Balances    │ ───► │ Differences │ ───► │ Lock Period │         │
│  │             │      │ (Adjustments│      │             │         │
│  └─────────────┘      └─────────────┘      └─────────────┘         │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Principles

- **GenAI for parsing**: Statement extraction via genai-service converts any format to structured JSON
- **Rules + manual for matching**: Primary matching via rule engine and user actions (not AI)
- **Staging workflow**: Parse → Review → Commit with human approval before matching
- **Accuracy**: Precise matching with user control
- **Auditability**: Full trail of parsing corrections, matches, and adjustments
- **Multi-format**: PDF, CSV, images - any bank format parsed by GenAI
- **Multi-tenant**: Complete isolation via tenant_id

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- GenAI integration for statement parsing
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-bank-accounts](../stories/002-bank-accounts.md) - RegisterBankAccount, GetBankAccount, ListBankAccounts, UpdateBankAccount

### Phase 2: Statement Parsing

- [ ] [003-statement-import](../stories/003-statement-import.md) - GenAI parsing, staging, review, commit workflow

### Phase 3: Matching (Core)

- [ ] [004-matching-rules](../stories/004-matching-rules.md) - CreateRule, ListRules, UpdateRule, DeleteRule, rule-based auto-matching
- [ ] [005-transaction-matching](../stories/005-transaction-matching.md) - Manual matching, split matching, exclusions

### Phase 4: Reconciliation

- [ ] [007-reconciliation-process](../stories/007-reconciliation-process.md) - Start, get, list, complete, abandon reconciliation
- [ ] [008-adjustments](../stories/008-adjustments.md) - Create adjustment entries, list adjustments, discrepancy resolution
- [ ] [009-observability](../stories/009-observability.md) - Metrics, tracing, structured logging

### Phase 5: Optional Enhancements

- [ ] [006-ai-matching](../stories/006-ai-matching.md) - Optional AI suggestions for difficult matches (low priority)

## gRPC Service

| Method | Story | Description |
|--------|-------|-------------|
| RegisterBankAccount | 002 | Create bank account linked to ledger |
| GetBankAccount | 002 | Get bank account details |
| ListBankAccounts | 002 | List tenant's bank accounts |
| UpdateBankAccount | 002 | Update bank account metadata |
| ImportStatement | 003 | Upload and trigger GenAI parsing |
| GetStatement | 003 | Get statement with parsing status |
| ListStatements | 003 | List statements for bank account |
| GetStagedTransactions | 003 | Get parsed transactions for review |
| UpdateStagedTransaction | 003 | Correct parsed transaction fields |
| CommitStatement | 003 | Approve and commit parsed transactions |
| AbandonStatement | 003 | Discard parsing, allow re-import |
| CreateMatchingRule | 004 | Create pattern-based matching rule |
| GetMatchingRule | 004 | Get single matching rule by ID |
| ListMatchingRules | 004 | List tenant's matching rules |
| UpdateMatchingRule | 004 | Update rule pattern/priority/status |
| DeleteMatchingRule | 004 | Remove matching rule |
| MatchTransaction | 005 | Manually match bank txn to ledger entries |
| UnmatchTransaction | 005 | Remove match from bank transaction |
| ExcludeTransaction | 005 | Exclude transaction from matching |
| GetCandidateEntries | 005 | Get ledger entries as match candidates |
| StartReconciliation | 007 | Begin reconciliation for period |
| GetReconciliation | 007 | Get reconciliation status and counts |
| ListReconciliations | 007 | List reconciliations for bank account |
| CompleteReconciliation | 007 | Finalize and lock reconciliation |
| AbandonReconciliation | 007 | Cancel in-progress reconciliation |
| CreateAdjustment | 008 | Create adjustment entry for discrepancy |
| ListAdjustments | 008 | List adjustments for reconciliation |
| GetAiSuggestions | 006 | (Optional) Get AI-suggested matches |
| ConfirmSuggestion | 006 | (Optional) Accept AI suggestion |
| RejectSuggestion | 006 | (Optional) Reject AI suggestion |

## Business Rules

From the spec:

1. Statement periods must not overlap for same account
2. Reconciliation locks period - no ledger changes allowed in locked periods
3. AI suggestions require user confirmation (no auto-matching by AI)
4. Matching rules are evaluated in priority order; first match wins
5. Excluded transactions still count toward balance verification
6. Split matches must sum to exact bank transaction amount
7. Completed reconciliations are immutable; corrections require new entries
8. Statement parsing failures mark individual transactions, not entire statement

## Dependencies

- **ledger-service**: Query balances and entries, create adjustments
- **document-service**: Store and retrieve statement files
- **genai-service**: Parse bank statements to structured JSON

## Acceptance Criteria

### Core (Must Have)
- [ ] GenAI parsing works for PDF, CSV, images
- [ ] Staging workflow: parse → review → commit
- [ ] User can correct parsed data before commit
- [ ] Rule-based auto-matching works correctly
- [ ] Manual matching works (match, exclude, split)
- [ ] Reconciliation compares ledger vs statement balances
- [ ] Reconciliation locks completed periods
- [ ] Adjustments create ledger entries
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed

### Optional (Nice to Have)
- [ ] AI suggestions available for difficult matches
- [ ] AI suggestions require user confirmation
