# Epic: Billing Service

Status: planning
Created: 2026-01-23

## Overview

Recurring billing and subscription management service for multi-tenant operations. Handles billing plans, subscriptions, usage tracking, and automated billing runs. Integrates with invoicing-service for invoice generation.

## Core Principles

- Subscription-first: All recurring revenue through subscriptions
- Usage metering: Support for consumption-based billing
- Proration: Fair charges for mid-cycle changes
- Idempotent: Billing runs are safe to retry
- Audit trail: Full history of billing events

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-billing-plans](../stories/002-billing-plans.md) - CreatePlan, GetPlan, ListPlans, ArchivePlan

### Phase 2: Subscriptions

- [ ] [003-subscription-management](../stories/003-subscription-management.md) - CreateSubscription, GetSubscription, ListSubscriptions
- [ ] [004-subscription-lifecycle](../stories/004-subscription-lifecycle.md) - Activate, Pause, Resume, Cancel

### Phase 3: Usage & Billing

- [ ] [005-usage-tracking](../stories/005-usage-tracking.md) - RecordUsage, GetUsage, usage aggregation
- [ ] [006-billing-runs](../stories/006-billing-runs.md) - RunBilling, charge calculation, invoice creation
- [ ] [007-proration](../stories/007-proration.md) - Mid-cycle changes, upgrade/downgrade

### Phase 4: Production

- [ ] [008-observability](../stories/008-observability.md) - Metrics, tracing, structured logging

## gRPC Service

| Method | Story |
|--------|-------|
| CreatePlan | 002 |
| GetPlan | 002 |
| ListPlans | 002 |
| ArchivePlan | 002 |
| CreateSubscription | 003 |
| GetSubscription | 003 |
| ListSubscriptions | 003 |
| ActivateSubscription | 004 |
| PauseSubscription | 004 |
| ResumeSubscription | 004 |
| CancelSubscription | 004 |
| ChangePlan | 004 |
| RecordUsage | 005 |
| GetUsage | 005 |
| RunBilling | 006 |
| GetBillingRun | 006 |
| ListBillingRuns | 006 |

## Acceptance Criteria

- [ ] All gRPC methods implemented and tested
- [ ] Subscription lifecycle enforced
- [ ] Usage correctly aggregated per billing cycle
- [ ] Billing runs create invoices via invoicing-service
- [ ] Proration calculated correctly
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed
- [ ] Health and readiness endpoints working
