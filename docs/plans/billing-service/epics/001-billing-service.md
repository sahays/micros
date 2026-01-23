# Epic: Billing Service

Status: planning
Created: 2026-01-23
Updated: 2026-01-23

## Overview

Recurring billing and subscription management service for multi-tenant operations. Manages subscriptions, billing plans, usage tracking, billing cycles, and charge calculation. Integrates with invoicing-service for invoice generation.

## Core Principles

- Billing anchor: Subscriptions bill on consistent anchor dates
- Usage aggregation: Metered usage collected and invoiced per cycle
- Proration: Partial period charges for mid-cycle changes
- Multi-tenant: Complete isolation via tenant_id
- Invoicing integration: Creates invoices via invoicing-service

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- Prometheus metrics, OpenTelemetry tracing
- service-core for shared infrastructure

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-plan-management](../stories/002-plan-management.md) - CreatePlan, GetPlan, UpdatePlan, ListPlans, ArchivePlan

### Phase 2: Subscriptions

- [ ] [003-subscription-crud](../stories/003-subscription-crud.md) - CreateSubscription, GetSubscription, ListSubscriptions
- [ ] [004-subscription-lifecycle](../stories/004-subscription-lifecycle.md) - Activate, Pause, Resume, Cancel subscription
- [ ] [005-plan-changes](../stories/005-plan-changes.md) - ChangePlan with proration support

### Phase 3: Usage

- [ ] [006-usage-tracking](../stories/006-usage-tracking.md) - RecordUsage, GetUsage, ListUsage with idempotency

### Phase 4: Billing

- [ ] [007-billing-cycles](../stories/007-billing-cycles.md) - Billing cycle management
- [ ] [008-billing-runs](../stories/008-billing-runs.md) - Batch and on-demand billing execution
- [ ] [009-charges](../stories/009-charges.md) - Charge management and one-time charges

### Phase 5: Observability

- [ ] [010-observability](../stories/010-observability.md) - Metrics, tracing, structured logging

## gRPC Service: BillingService

| Method | Story | Status |
|--------|-------|--------|
| CreatePlan | 002 | Planned |
| GetPlan | 002 | Planned |
| UpdatePlan | 002 | Planned |
| ListPlans | 002 | Planned |
| ArchivePlan | 002 | Planned |
| CreateSubscription | 003 | Planned |
| GetSubscription | 003 | Planned |
| ListSubscriptions | 003 | Planned |
| ActivateSubscription | 004 | Planned |
| PauseSubscription | 004 | Planned |
| ResumeSubscription | 004 | Planned |
| CancelSubscription | 004 | Planned |
| ChangePlan | 005 | Planned |
| RecordUsage | 006 | Planned |
| GetUsage | 006 | Planned |
| ListUsage | 006 | Planned |
| GetUsageSummary | 006 | Planned |
| GetBillingCycle | 007 | Planned |
| ListBillingCycles | 007 | Planned |
| AdvanceBillingCycle | 007 | Planned |
| RunBilling | 008 | Planned |
| RunBillingForSubscription | 008 | Planned |
| GetBillingRun | 008 | Planned |
| ListBillingRuns | 008 | Planned |
| GetCharge | 009 | Planned |
| ListCharges | 009 | Planned |
| CreateOneTimeCharge | 009 | Planned |

## Capabilities

| Capability | Methods | Description |
|------------|---------|-------------|
| `billing.plan:create` | CreatePlan | Create billing plans |
| `billing.plan:read` | GetPlan, ListPlans | View billing plans |
| `billing.plan:update` | UpdatePlan, ArchivePlan | Modify billing plans |
| `billing.subscription:create` | CreateSubscription | Create subscriptions |
| `billing.subscription:read` | GetSubscription, ListSubscriptions | View subscriptions |
| `billing.subscription:manage` | Activate, Pause, Resume, Cancel | Manage subscription lifecycle |
| `billing.subscription:change` | ChangePlan | Change subscription plan |
| `billing.usage:write` | RecordUsage | Record usage events |
| `billing.usage:read` | GetUsage, ListUsage, GetUsageSummary | View usage data |
| `billing.cycle:read` | GetBillingCycle, ListBillingCycles, GetCharge, ListCharges | View billing cycles and charges |
| `billing.cycle:manage` | AdvanceBillingCycle | Manage billing cycles |
| `billing.charge:create` | CreateOneTimeCharge | Create ad-hoc charges |
| `billing.run:execute` | RunBilling, RunBillingForSubscription | Execute billing runs |
| `billing.run:read` | GetBillingRun, ListBillingRuns | View billing run results |

## Metering

Per-tenant billing metrics:
- `billing_subscriptions_total{tenant_id, status}` - Subscription count by status
- `billing_plans_total{tenant_id}` - Plan count per tenant
- `billing_usage_records_total{tenant_id, component}` - Usage records by component
- `billing_runs_total{tenant_id, status}` - Billing run count by status
- `billing_charges_amount_total{tenant_id, currency}` - Total charged amount

## Dependencies

- **invoicing-service**: Create invoices for billing cycles
- **ledger-service**: Indirect, via invoicing-service
- **notification-service**: Billing reminders, payment failures (optional)

## Acceptance Criteria

- [ ] Project scaffolding complete
- [ ] PostgreSQL schema and migrations working
- [ ] Plan CRUD operations
- [ ] Subscription CRUD and lifecycle management
- [ ] Usage tracking with idempotency
- [ ] Billing cycle management
- [ ] Billing run execution
- [ ] Proration calculations
- [ ] Invoicing-service integration
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed
- [ ] Health and readiness endpoints working
- [ ] OpenTelemetry tracing configured
- [ ] All integration tests passing

## Architecture Notes

### Invoicing Integration

The billing-service integrates with invoicing-service via gRPC:

1. **service-core/src/grpc/invoicing_client.rs** - Reusable InvoicingClient
2. Billing run creates invoices for each subscription's charges
3. Connection is optional - billing run fails gracefully if invoicing-service unavailable

### Billing Flow

```
1. Subscription created → Initial billing cycle created
2. Usage recorded → Aggregated in current cycle
3. Billing run triggered (scheduled or manual)
4. For each due subscription:
   a. Calculate recurring charges from plan
   b. Calculate usage charges from aggregated usage
   c. Apply proration if applicable
   d. Create invoice via invoicing-service
   e. Mark cycle as invoiced
5. Track billing run results
```

### Proration Modes

- **immediate**: Charges applied immediately, prorated for partial period
- **next_cycle**: Changes take effect at next billing cycle
- **none**: No proration, full charge applies
