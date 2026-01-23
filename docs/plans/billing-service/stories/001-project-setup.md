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
- [ ] Create config module with environment loading (including AuthConfig)
- [ ] Create database connection pool
- [ ] Create migration 001 with all tables
- [ ] Create HTTP server with /health, /ready, /metrics endpoints
- [ ] Create gRPC server skeleton with interceptors
- [ ] Add capability_check.rs module
- [ ] Add metrics.rs module for billing-specific metrics
- [ ] Add invoicing-service client for integration
- [ ] Add to docker-compose with PostgreSQL dependency
- [ ] Update .env.example with billing-service config
- [ ] Add to scripts/pre-commit.sh ALL_SERVICES
- [ ] Add to scripts/integ-tests.sh PG_SERVICES

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
| tax_rate_id | UUID | FK |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE |
| is_archived | BOOLEAN | NOT NULL DEFAULT FALSE |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |
| updated_utc | TIMESTAMPTZ | NOT NULL |

### usage_components
| Column | Type | Constraints |
|--------|------|-------------|
| component_id | UUID | PK |
| plan_id | UUID | FK → billing_plans |
| name | VARCHAR(100) | NOT NULL |
| unit_name | VARCHAR(50) | NOT NULL (e.g., "API calls", "GB") |
| unit_price | DECIMAL(19,4) | NOT NULL |
| included_units | INTEGER | NOT NULL DEFAULT 0 |
| is_active | BOOLEAN | NOT NULL DEFAULT TRUE |

### subscriptions
| Column | Type | Constraints |
|--------|------|-------------|
| subscription_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| customer_id | UUID | NOT NULL |
| plan_id | UUID | FK → billing_plans |
| status | VARCHAR(20) | NOT NULL (active, paused, cancelled, expired) |
| billing_anchor_day | INTEGER | NOT NULL (1-31) |
| start_date | DATE | NOT NULL |
| end_date | DATE | |
| trial_end_date | DATE | |
| current_period_start | DATE | NOT NULL |
| current_period_end | DATE | NOT NULL |
| proration_mode | VARCHAR(20) | NOT NULL DEFAULT 'immediate' |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |
| updated_utc | TIMESTAMPTZ | NOT NULL |

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
| updated_utc | TIMESTAMPTZ | NOT NULL |

### charges
| Column | Type | Constraints |
|--------|------|-------------|
| charge_id | UUID | PK |
| cycle_id | UUID | FK → billing_cycles |
| charge_type | VARCHAR(20) | NOT NULL (recurring, usage, one_time) |
| description | TEXT | NOT NULL |
| quantity | DECIMAL(19,4) | NOT NULL |
| unit_price | DECIMAL(19,4) | NOT NULL |
| amount | DECIMAL(19,4) | NOT NULL |
| is_prorated | BOOLEAN | NOT NULL DEFAULT FALSE |
| proration_factor | DECIMAL(10,6) | |
| component_id | UUID | FK → usage_components |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### usage_records
| Column | Type | Constraints |
|--------|------|-------------|
| record_id | UUID | PK |
| subscription_id | UUID | FK → subscriptions |
| component_id | UUID | FK → usage_components |
| idempotency_key | VARCHAR(255) | NOT NULL, UNIQUE |
| quantity | DECIMAL(19,4) | NOT NULL |
| timestamp | TIMESTAMPTZ | NOT NULL |
| cycle_id | UUID | FK → billing_cycles |
| is_invoiced | BOOLEAN | NOT NULL DEFAULT FALSE |
| metadata | JSONB | |
| created_utc | TIMESTAMPTZ | NOT NULL |

### billing_runs
| Column | Type | Constraints |
|--------|------|-------------|
| run_id | UUID | PK |
| tenant_id | UUID | NOT NULL |
| run_type | VARCHAR(20) | NOT NULL (scheduled, manual, single) |
| status | VARCHAR(20) | NOT NULL (running, completed, failed) |
| started_utc | TIMESTAMPTZ | NOT NULL |
| completed_utc | TIMESTAMPTZ | |
| subscriptions_processed | INTEGER | NOT NULL DEFAULT 0 |
| subscriptions_succeeded | INTEGER | NOT NULL DEFAULT 0 |
| subscriptions_failed | INTEGER | NOT NULL DEFAULT 0 |
| error_message | TEXT | |

### billing_run_results
| Column | Type | Constraints |
|--------|------|-------------|
| result_id | UUID | PK |
| run_id | UUID | FK → billing_runs |
| subscription_id | UUID | FK → subscriptions |
| status | VARCHAR(20) | NOT NULL (success, failed) |
| invoice_id | UUID | |
| error_message | TEXT | |
| created_utc | TIMESTAMPTZ | NOT NULL |

## Capabilities

```rust
pub mod capabilities {
    pub const BILLING_PLAN_CREATE: &str = "billing.plan:create";
    pub const BILLING_PLAN_READ: &str = "billing.plan:read";
    pub const BILLING_PLAN_UPDATE: &str = "billing.plan:update";
    pub const BILLING_SUBSCRIPTION_CREATE: &str = "billing.subscription:create";
    pub const BILLING_SUBSCRIPTION_READ: &str = "billing.subscription:read";
    pub const BILLING_SUBSCRIPTION_MANAGE: &str = "billing.subscription:manage";
    pub const BILLING_SUBSCRIPTION_CHANGE: &str = "billing.subscription:change";
    pub const BILLING_USAGE_WRITE: &str = "billing.usage:write";
    pub const BILLING_USAGE_READ: &str = "billing.usage:read";
    pub const BILLING_CYCLE_READ: &str = "billing.cycle:read";
    pub const BILLING_CYCLE_MANAGE: &str = "billing.cycle:manage";
    pub const BILLING_CHARGE_CREATE: &str = "billing.charge:create";
    pub const BILLING_RUN_EXECUTE: &str = "billing.run:execute";
    pub const BILLING_RUN_READ: &str = "billing.run:read";
}
```

## Acceptance Criteria

- [ ] `cargo build -p billing-service` succeeds
- [ ] `sqlx migrate run` creates all tables
- [ ] Health endpoint returns 200
- [ ] Metrics endpoint returns Prometheus format
- [ ] gRPC reflection lists BillingService
- [ ] Docker container starts and connects to PostgreSQL
- [ ] Service included in pre-commit and integ-tests scripts

## Integration Tests

- [ ] Health endpoint returns service name and version
- [ ] Readiness fails when database unavailable
- [ ] gRPC health check returns SERVING
- [ ] Metrics endpoint returns valid Prometheus format
