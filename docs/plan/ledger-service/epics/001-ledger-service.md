# Epic: Ledger Service

Status: planning
Created: 2026-01-22

## Overview

Generic double-entry accounting microservice for multi-tenant financial operations. Reusable across apartments, schools, marketplaces, and wallet applications.

## Core Principles

- Double-entry: Every transaction has equal debits and credits
- Immutability: Append-only, no updates or deletes
- Consistency: Balances derived from entries, never stored
- Multi-tenant: Complete isolation via tenant_id
- Idempotent: Safe retries via idempotency_key

## Tech Stack

- Rust + Axum (HTTP health/metrics) + Tonic (gRPC)
- PostgreSQL + sqlx (compile-time checked queries)
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-account-management](../stories/002-account-management.md) - CreateAccount, GetAccount, ListAccounts
- [ ] [003-transaction-posting](../stories/003-transaction-posting.md) - PostTransaction with double-entry validation

### Phase 2: Queries

- [ ] [004-balance-queries](../stories/004-balance-queries.md) - GetBalance, GetBalances, point-in-time
- [ ] [005-transaction-queries](../stories/005-transaction-queries.md) - GetTransaction, ListTransactions
- [ ] [006-statements](../stories/006-statements.md) - GetStatement with date range

### Phase 3: Production

- [ ] [007-idempotency](../stories/007-idempotency.md) - Idempotency keys, concurrency control
- [ ] [008-observability](../stories/008-observability.md) - Metrics, tracing, structured logging

## gRPC Service

| Method | Story |
|--------|-------|
| CreateAccount | 002 |
| GetAccount | 002 |
| ListAccounts | 002 |
| PostTransaction | 003 |
| GetTransaction | 005 |
| ListTransactions | 005 |
| GetBalance | 004 |
| GetBalances | 004 |
| GetStatement | 006 |

## Acceptance Criteria

- [ ] All gRPC methods implemented and tested
- [ ] Double-entry constraint enforced at database level
- [ ] Idempotency prevents duplicate transactions
- [ ] Point-in-time balance queries work correctly
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed
- [ ] Tracing spans for all operations
- [ ] Health and readiness endpoints working
