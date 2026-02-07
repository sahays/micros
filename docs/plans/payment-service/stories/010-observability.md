# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement comprehensive observability for all new SaaS payment platform domains including structured logging, distributed tracing, and Prometheus metrics for linked accounts, transfers, settlements, subscriptions, customers, payment links, and refunds.

## Tasks

- [ ] Add structured logging for linked account lifecycle events
- [ ] Add structured logging for transfer and settlement operations
- [ ] Add structured logging for subscription lifecycle events
- [ ] Add structured logging for payment link and refund operations
- [ ] Add OpenTelemetry spans for all new gRPC methods
- [ ] Add trace context propagation to Razorpay API calls for new domains
- [ ] Create Prometheus metrics for linked accounts
- [ ] Create Prometheus metrics for transfers and commissions
- [ ] Create Prometheus metrics for settlements
- [ ] Create Prometheus metrics for subscriptions
- [ ] Create Prometheus metrics for refunds
- [ ] Create Prometheus metrics for payment links
- [ ] Create Prometheus metrics for webhook events by domain
- [ ] Update Grafana dashboard with new payment platform panels

## Structured Logging

All logs formatted as JSON for Loki ingestion:

```json
{
  "timestamp": "2026-02-07T10:30:00Z",
  "level": "info",
  "msg": "Transfer processed to linked account",
  "service": "payment-service",
  "trace_id": "abc123",
  "span_id": "def456",
  "tenant_id": "tenant-uuid",
  "transfer_id": "transfer-uuid",
  "linked_account_id": "la-uuid",
  "amount": 9500,
  "commission": 500,
  "currency": "INR"
}
```

**Log Levels:**
- `debug`: Detailed processing (commission calculation, webhook parsing)
- `info`: Normal operations (transfer created, subscription activated, settlement processed)
- `warn`: Recoverable issues (webhook retry, settlement hold, subscription pending)
- `error`: Failures (transfer failed, refund failed, Razorpay API error)

## Distributed Tracing

**Trace Context Propagation:**
- gRPC interceptor extracts/injects trace headers
- Propagate to Razorpay API calls for all new domains
- All MongoDB operations as spans

**Key Spans:**
- `payment.linked_account.create` - Linked account creation with Razorpay Route
- `payment.transfer.create` - Transfer creation with commission calculation
- `payment.transfer.reverse` - Transfer reversal
- `payment.settlement.request` - On-demand settlement request
- `payment.subscription.create` - Subscription creation
- `payment.subscription.charge` - Subscription charge processing
- `payment.refund.initiate` - Refund initiation with transfer reversal
- `payment.payment_link.create` - Payment link creation
- `payment.webhook.process` - Webhook event processing by domain

## Prometheus Metrics

### Linked Account Metrics
```rust
pub static PAYMENT_LINKED_ACCOUNTS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (created, under_review, activated, suspended)

pub static PAYMENT_LINKED_ACCOUNT_ACTIVATIONS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id
```

### Transfer Metrics
```rust
pub static PAYMENT_TRANSFERS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (created, processed, reversed, failed)

pub static PAYMENT_TRANSFER_AMOUNT_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, currency

pub static PAYMENT_TRANSFER_REVERSALS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id
```

### Commission Metrics
```rust
pub static PAYMENT_COMMISSION_AMOUNT_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, currency

pub static PAYMENT_COMMISSION_CONFIG_UPDATES_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id
```

### Settlement Metrics
```rust
pub static PAYMENT_SETTLEMENTS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, type (normal, instant, on_demand), status (created, processed, failed)

pub static PAYMENT_SETTLEMENT_AMOUNT_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, currency
```

### Subscription Metrics
```rust
pub static PAYMENT_SUBSCRIPTIONS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (created, authenticated, active, pending, halted, paused, cancelled, completed)

pub static PAYMENT_SUBSCRIPTION_CHARGES_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (success, failed)

pub static PAYMENT_SUBSCRIPTION_RETRIES_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id
```

### Refund Metrics
```rust
pub static PAYMENT_REFUNDS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (created, processed, failed), speed (normal, optimum)

pub static PAYMENT_REFUND_AMOUNT_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, currency
```

### Payment Link Metrics
```rust
pub static PAYMENT_LINKS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (created, partially_paid, paid, cancelled, expired)

pub static PAYMENT_LINK_AMOUNT_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, currency
```

### Webhook Metrics
```rust
pub static PAYMENT_WEBHOOK_EVENTS_TOTAL: Counter<Labels> = ...;
  // Labels: event_type, domain (payment, transfer, settlement, subscription, account, payment_link, refund)
```

## Grafana Dashboard

Updated dashboard panels:
- Linked account status distribution (gauge)
- Transfer volume and success rate
- Commission collected over time
- Settlement volume by type
- Subscription status distribution (gauge)
- Subscription charge success/failure rate
- Refund volume and speed distribution
- Payment link conversion rate (created → paid)
- Webhook event volume by domain
- Razorpay API call latency by domain

## Acceptance Criteria

- [ ] Logs are JSON formatted with trace context for all new domains
- [ ] Traces appear in Tempo with full span hierarchy for new operations
- [ ] Traces propagate to Razorpay API calls for all domains
- [ ] Linked account metrics available at /metrics
- [ ] Transfer and commission metrics available at /metrics
- [ ] Settlement metrics available at /metrics
- [ ] Subscription metrics available at /metrics
- [ ] Refund metrics available at /metrics
- [ ] Payment link metrics available at /metrics
- [ ] Webhook event metrics include domain label
- [ ] Grafana dashboard shows new payment platform panels

## Integration Tests

- [ ] Metrics endpoint includes payment_linked_accounts_total
- [ ] Metrics endpoint includes payment_transfers_total
- [ ] Metrics endpoint includes payment_commission_amount_total
- [ ] Metrics endpoint includes payment_settlements_total
- [ ] Metrics endpoint includes payment_subscriptions_total
- [ ] Metrics endpoint includes payment_refunds_total
- [ ] Metrics endpoint includes payment_links_total
- [ ] Metrics endpoint includes payment_webhook_events_total with domain label
