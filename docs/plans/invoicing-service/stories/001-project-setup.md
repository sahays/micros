# Story: Project Setup

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Scaffold invoicing-service with PostgreSQL, sqlx migrations, gRPC server, and HTTP health endpoints.

## Tasks

- [ ] Create `invoicing-service/` directory with Cargo.toml
- [ ] Add workspace member to root Cargo.toml
- [ ] Add dependencies: sqlx, tonic, axum, tokio, tracing, prometheus
- [ ] Create proto definition `proto/micros/invoicing/v1/invoicing.proto`
- [ ] Create config module with environment loading
- [ ] Create database connection pool
- [ ] Create migration 001 with invoices, line_items, tax_rates, receipts tables
- [ ] Create HTTP server with /health, /ready, /metrics endpoints
- [ ] Create gRPC server skeleton
- [ ] Add ledger-service client for integration
- [ ] Add to docker-compose with PostgreSQL dependency
- [ ] Update .env.example with invoicing-service config

## Schema

### invoices
| Column | Type | Constraints |
|--------|------|-------------|
| invoice_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| invoice_number | VARCHAR(50) | NOT NULL, UNIQUE(tenant_id, invoice_number) |
| invoice_type | VARCHAR(20) | NOT NULL (standard, credit_note, proforma) |
| status | VARCHAR(20) | NOT NULL (draft, issued, paid, void, overdue) |
| customer_id | UUID | NOT NULL |
| customer_name | VARCHAR(255) | NOT NULL |
| customer_address | JSONB | |
| issue_date | DATE | |
| due_date | DATE | NOT NULL |
| currency | VARCHAR(3) | NOT NULL |
| subtotal | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| tax_total | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| total | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| amount_paid | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| balance_due | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| notes | TEXT | |
| terms | TEXT | |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |
| updated_utc | TIMESTAMPTZ | NOT NULL |

### invoice_line_items
| Column | Type | Constraints |
|--------|------|-------------|
| line_item_id | UUID | PK |
| invoice_id | UUID | FK → invoices |
| line_number | INTEGER | NOT NULL |
| description | TEXT | NOT NULL |
| quantity | DECIMAL(19,4) | NOT NULL |
| unit_price | DECIMAL(19,4) | NOT NULL |
| amount | DECIMAL(19,4) | NOT NULL |
| tax_rate_id | UUID | FK → tax_rates |
| tax_amount | DECIMAL(19,4) | NOT NULL DEFAULT 0 |
| account_id | UUID | (ledger account for revenue) |
| metadata | JSONB | |

### tax_rates
| Column | Type | Constraints |
|--------|------|-------------|
| tax_rate_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| name | VARCHAR(100) | NOT NULL |
| rate | DECIMAL(10,6) | NOT NULL |
| tax_type | VARCHAR(20) | NOT NULL (inclusive, exclusive) |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE |
| effective_from | DATE | NOT NULL |
| effective_to | DATE | |

### receipts
| Column | Type | Constraints |
|--------|------|-------------|
| receipt_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| receipt_number | VARCHAR(50) | NOT NULL, UNIQUE(tenant_id, receipt_number) |
| invoice_id | UUID | FK → invoices |
| amount | DECIMAL(19,4) | NOT NULL |
| payment_method | VARCHAR(50) | NOT NULL |
| payment_reference | VARCHAR(255) | |
| received_date | DATE | NOT NULL |
| journal_id | UUID | (ledger entry reference) |
| notes | TEXT | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### invoice_sequences
| Column | Type | Constraints |
|--------|------|-------------|
| tenant_id | UUID | PK |
| year_month | VARCHAR(7) | PK (YYYY-MM) |
| last_sequence | INTEGER | NOT NULL DEFAULT 0 |

## Acceptance Criteria

- [ ] `cargo build -p invoicing-service` succeeds
- [ ] `sqlx migrate run` creates tables
- [ ] Health endpoint returns 200
- [ ] gRPC reflection lists InvoicingService
- [ ] Docker container starts and connects to PostgreSQL

## Integration Tests

- [ ] Health endpoint returns service name and version
- [ ] Readiness fails when database unavailable
- [ ] gRPC health check returns SERVING
