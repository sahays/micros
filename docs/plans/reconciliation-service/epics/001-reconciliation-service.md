# Epic: Reconciliation Service

Status: planning
Created: 2026-01-23

## Overview

Bank statement reconciliation service with AI-powered transaction matching. Parses bank statements, matches transactions to ledger entries, and identifies discrepancies. Integrates with genai-service for intelligent matching.

## Core Principles

- GenAI-first: Statement extraction via genai-service, no bank-specific parsers
- Staging workflow: Extract → Review → Commit with human approval
- Accuracy: Precise matching with confidence scores
- Auditability: Full trail of extractions, corrections, matches, and adjustments
- AI-assisted: GenAI suggestions with human confirmation
- Multi-format: PDF, CSV, images - any bank format
- Multi-tenant: Complete isolation via tenant_id

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- GenAI integration for matching
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-bank-accounts](../stories/002-bank-accounts.md) - RegisterBankAccount, GetBankAccount, ListBankAccounts

### Phase 2: Statement Import

- [ ] [003-statement-import](../stories/003-statement-import.md) - GenAI extraction, staging, review, commit workflow
- [ ] [004-matching-rules](../stories/004-matching-rules.md) - CreateRule, ListRules, rule-based auto-matching

### Phase 3: Matching

- [ ] [005-transaction-matching](../stories/005-transaction-matching.md) - Manual matching, split matching, exclusions
- [ ] [006-ai-matching](../stories/006-ai-matching.md) - GenAI suggestions, confidence scores, learning

### Phase 4: Reconciliation

- [ ] [007-reconciliation-process](../stories/007-reconciliation-process.md) - Start, complete, abandon reconciliation
- [ ] [008-adjustments](../stories/008-adjustments.md) - Create adjustment entries, discrepancy resolution
- [ ] [009-observability](../stories/009-observability.md) - Metrics, tracing, structured logging

## gRPC Service

| Method | Story |
|--------|-------|
| RegisterBankAccount | 002 |
| GetBankAccount | 002 |
| ListBankAccounts | 002 |
| ImportStatement | 003 |
| GetStatement | 003 |
| ListStatements | 003 |
| GetStagedTransactions | 003 |
| UpdateStagedTransaction | 003 |
| CommitStatement | 003 |
| AbandonStatement | 003 |
| CreateMatchingRule | 004 |
| ListMatchingRules | 004 |
| DeleteMatchingRule | 004 |
| MatchTransaction | 005 |
| UnmatchTransaction | 005 |
| ExcludeTransaction | 005 |
| GetAiSuggestions | 006 |
| ConfirmSuggestion | 006 |
| RejectSuggestion | 006 |
| StartReconciliation | 007 |
| GetReconciliation | 007 |
| CompleteReconciliation | 007 |
| AbandonReconciliation | 007 |
| CreateAdjustment | 008 |

## Acceptance Criteria

- [ ] All gRPC methods implemented and tested
- [ ] GenAI extraction works for PDF, CSV, images
- [ ] Staging workflow: extract → review → commit
- [ ] User can correct extracted data before commit
- [ ] Rule-based matching works correctly
- [ ] AI suggestions have confidence scores
- [ ] Reconciliation locks matched periods
- [ ] Adjustments create ledger entries
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed
