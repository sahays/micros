# Story: Project Setup

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Scaffold reconciliation-service with PostgreSQL, sqlx migrations, gRPC server, and HTTP health endpoints.

## Tasks

- [ ] Create `reconciliation-service/` directory with Cargo.toml
- [ ] Add workspace member to root Cargo.toml
- [ ] Add dependencies: sqlx, tonic, axum, tokio, tracing, prometheus
- [ ] Create proto definition `proto/micros/reconciliation/v1/reconciliation.proto`
- [ ] Create config module with environment loading
- [ ] Create database connection pool
- [ ] Create migration 001 with bank_accounts, statements, transactions, matches tables
- [ ] Create HTTP server with /health, /ready, /metrics endpoints
- [ ] Create gRPC server skeleton
- [ ] Add ledger-service client for integration
- [ ] Add genai-service client for AI matching
- [ ] Add document-service client for statement files
- [ ] Add to docker-compose with PostgreSQL dependency
- [ ] Update .env.example with reconciliation-service config

## Schema

### bank_accounts
| Column | Type | Constraints |
|--------|------|-------------|
| bank_account_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| ledger_account_id | UUID | NOT NULL (ledger cash/bank account) |
| bank_name | VARCHAR(100) | NOT NULL |
| account_number_masked | VARCHAR(20) | NOT NULL (last 4 digits) |
| currency | VARCHAR(3) | NOT NULL |
| statement_format | VARCHAR(20) | NOT NULL (csv, ofx, mt940) |
| format_config | JSONB | (column mappings, etc.) |
| last_reconciled_date | DATE | |
| last_reconciled_balance | DECIMAL(19,4) | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### bank_statements
| Column | Type | Constraints |
|--------|------|-------------|
| statement_id | UUID | PK |
| bank_account_id | UUID | FK → bank_accounts |
| tenant_id | UUID | NOT NULL |
| document_id | UUID | (document-service reference) |
| period_start | DATE | NOT NULL |
| period_end | DATE | NOT NULL |
| opening_balance | DECIMAL(19,4) | NOT NULL |
| closing_balance | DECIMAL(19,4) | NOT NULL |
| status | VARCHAR(20) | NOT NULL (uploaded, parsing, parsed, reconciling, reconciled, failed) |
| error_message | TEXT | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### bank_transactions
| Column | Type | Constraints |
|--------|------|-------------|
| transaction_id | UUID | PK |
| statement_id | UUID | FK → bank_statements |
| tenant_id | UUID | NOT NULL |
| transaction_date | DATE | NOT NULL |
| description | TEXT | NOT NULL |
| reference | VARCHAR(100) | |
| amount | DECIMAL(19,4) | NOT NULL (positive=deposit, negative=withdrawal) |
| running_balance | DECIMAL(19,4) | |
| status | VARCHAR(20) | NOT NULL (unmatched, matched, manually_matched, excluded) |
| created_utc | TIMESTAMPTZ | NOT NULL |

### transaction_matches
| Column | Type | Constraints |
|--------|------|-------------|
| match_id | UUID | PK |
| bank_transaction_id | UUID | FK → bank_transactions |
| ledger_entry_id | UUID | NOT NULL (from ledger-service) |
| match_type | VARCHAR(20) | NOT NULL (auto, manual, ai_confirmed) |
| confidence_score | DECIMAL(5,4) | (0-1 for AI matches) |
| matched_by | VARCHAR(100) | (user or rule name) |
| matched_utc | TIMESTAMPTZ | NOT NULL |

### matching_rules
| Column | Type | Constraints |
|--------|------|-------------|
| rule_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| name | VARCHAR(100) | NOT NULL |
| description_pattern | VARCHAR(255) | NOT NULL (regex or contains) |
| match_type | VARCHAR(20) | NOT NULL (regex, contains, exact) |
| target_account_id | UUID | (ledger account to match) |
| priority | INTEGER | NOT NULL DEFAULT 0 |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE |
| created_utc | TIMESTAMPTZ | NOT NULL |

### reconciliations
| Column | Type | Constraints |
|--------|------|-------------|
| reconciliation_id | UUID | PK |
| bank_account_id | UUID | FK → bank_accounts |
| tenant_id | UUID | NOT NULL |
| period_start | DATE | NOT NULL |
| period_end | DATE | NOT NULL |
| expected_balance | DECIMAL(19,4) | NOT NULL (from ledger) |
| actual_balance | DECIMAL(19,4) | NOT NULL (from statement) |
| difference | DECIMAL(19,4) | NOT NULL |
| status | VARCHAR(20) | NOT NULL (in_progress, completed, abandoned) |
| started_utc | TIMESTAMPTZ | NOT NULL |
| completed_utc | TIMESTAMPTZ | |

## Acceptance Criteria

- [ ] `cargo build -p reconciliation-service` succeeds
- [ ] `sqlx migrate run` creates tables
- [ ] Health endpoint returns 200
- [ ] gRPC reflection lists ReconciliationService
- [ ] Docker container starts and connects to PostgreSQL

## Integration Tests

- [ ] Health endpoint returns service name and version
- [ ] Readiness fails when database unavailable
- [ ] gRPC health check returns SERVING
