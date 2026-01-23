# Story: Project Setup

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Scaffold billing-service with PostgreSQL, sqlx migrations, gRPC server, and HTTP health endpoints.

## Tasks

- [ ] Create `billing-service/` directory with Cargo.toml
- [ ] Add workspace member to root Cargo.toml
- [ ] Add dependencies: sqlx, tonic, axum, tokio, tracing, prometheus
- [ ] Create proto definition `proto/micros/billing/v1/billing.proto`
- [ ] Create config module with environment loading
- [ ] Create database connection pool
- [ ] Create migration 001 with plans, subscriptions, usage_records, billing_runs tables
- [ ] Create HTTP server with /health, /ready, /metrics endpoints
- [ ] Create gRPC server skeleton
- [ ] Add invoicing-service client for integration
- [ ] Add to docker-compose with PostgreSQL dependency
- [ ] Update .env.example with billing-service config

## Schema

### billing_plans
| Column | Type | Constraints |
|--------|------|-------------|
| plan_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| name | VARCHAR(100) | NOT NULL |
| description | TEXT | |
| billing_interval | VARCHAR(20) | NOT NULL (daily, weekly, monthly, quarterly, annually) |
| interval_count | INTEGER | NOT NULL DEFAULT 1 |
| base_price | DECIMAL(19,4) | NOT NULL |
| currency | VARCHAR(3) | NOT NULL |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |
| archived_utc | TIMESTAMPTZ | |

### plan_usage_components
| Column | Type | Constraints |
|--------|------|-------------|
| component_id | UUID | PK |
| plan_id | UUID | FK → billing_plans |
| name | VARCHAR(100) | NOT NULL |
| unit_name | VARCHAR(50) | NOT NULL (e.g., "API calls", "GB") |
| unit_price | DECIMAL(19,4) | NOT NULL |
| included_units | DECIMAL(19,4) | NOT NULL DEFAULT 0 |

### subscriptions
| Column | Type | Constraints |
|--------|------|-------------|
| subscription_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| customer_id | UUID | NOT NULL |
| plan_id | UUID | FK → billing_plans |
| status | VARCHAR(20) | NOT NULL (active, paused, cancelled, expired) |
| billing_anchor | DATE | NOT NULL |
| current_period_start | DATE | NOT NULL |
| current_period_end | DATE | NOT NULL |
| trial_end | DATE | |
| cancelled_at | TIMESTAMPTZ | |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### usage_records
| Column | Type | Constraints |
|--------|------|-------------|
| usage_id | UUID | PK |
| subscription_id | UUID | FK → subscriptions |
| component_id | UUID | FK → plan_usage_components |
| quantity | DECIMAL(19,4) | NOT NULL |
| timestamp | TIMESTAMPTZ | NOT NULL |
| idempotency_key | VARCHAR(255) | UNIQUE |

### billing_cycles
| Column | Type | Constraints |
|--------|------|-------------|
| cycle_id | UUID | PK |
| subscription_id | UUID | FK → subscriptions |
| period_start | DATE | NOT NULL |
| period_end | DATE | NOT NULL |
| status | VARCHAR(20) | NOT NULL (pending, invoiced, paid, failed) |
| invoice_id | UUID | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### billing_runs
| Column | Type | Constraints |
|--------|------|-------------|
| run_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| run_date | DATE | NOT NULL |
| status | VARCHAR(20) | NOT NULL (running, completed, failed) |
| total_subscriptions | INTEGER | NOT NULL DEFAULT 0 |
| successful | INTEGER | NOT NULL DEFAULT 0 |
| failed | INTEGER | NOT NULL DEFAULT 0 |
| started_utc | TIMESTAMPTZ | NOT NULL |
| completed_utc | TIMESTAMPTZ | |

## Acceptance Criteria

- [ ] `cargo build -p billing-service` succeeds
- [ ] `sqlx migrate run` creates tables
- [ ] Health endpoint returns 200
- [ ] gRPC reflection lists BillingService
- [ ] Docker container starts and connects to PostgreSQL

## Integration Tests

- [ ] Health endpoint returns service name and version
- [ ] Readiness fails when database unavailable
- [ ] gRPC health check returns SERVING
