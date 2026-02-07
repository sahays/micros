# Epic: Payment Service Integration

Status: planning
Created: 2026-02-07
Updated: 2026-02-07

## Overview

Integrate billing-service with payment-service for Razorpay subscription orchestration. Billing-service owns billing business logic (plans, proration, usage, billing cycles) and orchestrates payment-service for creating and managing Razorpay subscriptions, processing recurring charges, and reacting to subscription lifecycle events.

## Core Principles

- Billing-service orchestrates, payment-service executes: Business logic in billing, Razorpay primitives in payment
- Event-driven: React to subscription webhooks forwarded from payment-service
- Idempotent: Webhook handlers and billing operations are safely retriable
- Multi-tenant: Complete isolation via tenant_id

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- PaymentServiceClient (gRPC client to payment-service)
- Prometheus metrics, OpenTelemetry tracing
- service-core for shared infrastructure

## Stories

### Phase 6: Payment Integration

- [ ] [011-payment-service-client](../stories/011-payment-service-client.md) - Add PaymentServiceClient to billing-service
- [ ] [012-razorpay-subscription-orchestration](../stories/012-razorpay-subscription-orchestration.md) - Orchestrate Razorpay subscriptions via payment-service

## gRPC Integration

| Payment-Service Method | Billing Usage | Story |
|------------------------|---------------|-------|
| CreateRazorpayPlan | Billing run creates Razorpay plan for billing plan | 012 |
| CreateRazorpaySubscription | Billing creates subscription for customer enrollment | 012 |
| GetRazorpaySubscription | Billing queries subscription status | 012 |
| PauseRazorpaySubscription | Billing pauses subscription on billing pause | 012 |
| ResumeRazorpaySubscription | Billing resumes subscription on billing resume | 012 |
| CancelRazorpaySubscription | Billing cancels subscription on billing cancellation | 012 |

## Dependencies

- **payment-service**: Razorpay subscription and plan management
- **invoicing-service**: Invoice generation for subscription charges
- **notification-service**: Subscription lifecycle alerts

## Acceptance Criteria

- [ ] PaymentServiceClient configured and connected
- [ ] Billing run creates Razorpay plans via payment-service
- [ ] Billing creates Razorpay subscriptions via payment-service
- [ ] Billing reacts to subscription.charged event
- [ ] Billing reacts to subscription.halted event
- [ ] Billing pauses/resumes/cancels Razorpay subscriptions
- [ ] Trace context propagated to payment-service calls
- [ ] All integration tests passing

## Architecture Notes

### Integration Flow

```
Billing-service                    Payment-service               Razorpay
     │                                  │                           │
     ├─ CreateRazorpayPlan ───────────→ │ ─── Create Plan ────────→ │
     │                                  │ ←── razorpay_plan_id ──── │
     │ ←── plan_id ──────────────────── │                           │
     │                                  │                           │
     ├─ CreateRazorpaySubscription ──→  │ ─── Create Sub ─────────→ │
     │                                  │ ←── short_url ──────────── │
     │ ←── subscription_id, short_url ─ │                           │
     │                                  │                           │
     │                                  │ ←── subscription.charged ─ │
     │ ←── webhook forwarded ────────── │                           │
     ├─ Update billing cycle            │                           │
     ├─ Create invoice                  │                           │
     │                                  │                           │
     │                                  │ ←── subscription.halted ── │
     │ ←── webhook forwarded ────────── │                           │
     ├─ Mark subscription failed        │                           │
     ├─ Send notification               │                           │
```
