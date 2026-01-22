# Story: Project Setup

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Scaffold ledger-service with PostgreSQL, sqlx migrations, gRPC server, and HTTP health endpoints.

## Tasks

- [ ] Create `ledger-service/` directory with Cargo.toml
- [ ] Add workspace member to root Cargo.toml
- [ ] Add dependencies: sqlx, tonic, axum, tokio, tracing, prometheus
- [ ] Create proto definition `proto/micros/ledger/v1/ledger.proto`
- [ ] Create config module with environment loading
- [ ] Create database connection pool
- [ ] Create migration 001 with accounts and ledger_entries tables
- [ ] Create HTTP server with /health, /ready, /metrics endpoints
- [ ] Create gRPC server skeleton
- [ ] Add to docker-compose with PostgreSQL dependency
- [ ] Update .env.example with ledger-service config

## Schema

### accounts
| Column | Type | Constraints |
|--------|------|-------------|
| account_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| account_type | VARCHAR(20) | NOT NULL (asset, liability, equity, revenue, expense) |
| account_code | VARCHAR(100) | NOT NULL |
| currency | VARCHAR(3) | NOT NULL |
| allow_negative | BOOLEAN | DEFAULT FALSE |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |
| closed_utc | TIMESTAMPTZ | |

**Indexes:** UNIQUE(tenant_id, account_code), idx_accounts_tenant

### ledger_entries
| Column | Type | Constraints |
|--------|------|-------------|
| entry_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| journal_id | UUID | NOT NULL |
| account_id | UUID | FK → accounts |
| amount | DECIMAL(19,4) | NOT NULL, CHECK > 0 |
| direction | VARCHAR(6) | NOT NULL (debit, credit) |
| effective_date | DATE | NOT NULL |
| posted_utc | TIMESTAMPTZ | NOT NULL |
| idempotency_key | VARCHAR(255) | |
| metadata | JSONB | |

**Indexes:** idx_entries_journal, idx_entries_account_date, UNIQUE(idempotency_key) WHERE NOT NULL

## Acceptance Criteria

- [ ] `cargo build -p ledger-service` succeeds
- [ ] `sqlx migrate run` creates tables
- [ ] Health endpoint returns 200
- [ ] gRPC reflection lists LedgerService
- [ ] Docker container starts and connects to PostgreSQL

## Integration Tests

- [ ] Health endpoint returns service name and version
- [ ] Readiness fails when database unavailable
- [ ] gRPC health check returns SERVING
