# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement comprehensive observability for billing-service including structured logging, distributed tracing, Prometheus metrics, and health endpoints.

## Tasks

- [ ] Configure OpenTelemetry tracing to Tempo
- [ ] Add trace context propagation to invoicing-service calls
- [ ] Implement structured JSON logging for PLG stack
- [ ] Create billing-specific Prometheus metrics
- [ ] Add gRPC interceptors for tracing and metrics
- [ ] Create Grafana dashboard for billing-service
- [ ] Add health and readiness endpoints
- [ ] Add metrics endpoint

## Structured Logging

All logs formatted as JSON for Loki ingestion:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "info",
  "msg": "Billing run completed",
  "service": "billing-service",
  "trace_id": "abc123",
  "span_id": "def456",
  "tenant_id": "tenant-uuid",
  "run_id": "run-uuid",
  "subscriptions_processed": 150,
  "subscriptions_succeeded": 148,
  "subscriptions_failed": 2
}
```

**Log Levels:**
- `debug`: Detailed processing info (charge calculations, usage aggregation)
- `info`: Normal operations (subscription created, billing run started)
- `warn`: Recoverable issues (retry scheduled, rate limit hit)
- `error`: Failures (invoice creation failed, database error)

## Distributed Tracing

**Trace Context Propagation:**
- gRPC interceptor extracts/injects trace headers
- Propagate to invoicing-service calls
- All database operations as spans

**Key Spans:**
- `billing.run` - entire billing run
- `billing.subscription.process` - per-subscription processing
- `billing.charge.calculate` - charge calculation
- `billing.invoice.create` - invoicing-service call
- `billing.usage.aggregate` - usage aggregation

## Prometheus Metrics

### Billing-Specific Metrics

```rust
// Per-tenant metering for billing
pub static BILLING_PLAN_OPERATIONS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, operation (created, updated, archived)

pub static BILLING_SUBSCRIPTION_OPERATIONS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, operation (created, activated, paused, resumed, cancelled, plan_changed)

pub static BILLING_SUBSCRIPTION_STATUS: Gauge<Labels> = ...;
  // Labels: tenant_id, status (trial, active, paused, cancelled, expired)

pub static BILLING_USAGE_RECORDS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, component_id

pub static BILLING_RUN_DURATION_SECONDS: Histogram<Labels> = ...;
  // Labels: tenant_id, run_type

pub static BILLING_RUN_SUBSCRIPTIONS_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, status (processed, succeeded, failed)

pub static BILLING_CHARGES_CREATED_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, charge_type

pub static BILLING_INVOICES_CREATED_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id

pub static BILLING_PRORATION_TOTAL: Counter<Labels> = ...;
  // Labels: tenant_id, mode (immediate, next_cycle, none)
```

### Standard gRPC Metrics

```rust
pub static GRPC_REQUESTS_TOTAL: Counter<Labels> = ...;
  // Labels: method, status

pub static GRPC_REQUEST_DURATION_SECONDS: Histogram<Labels> = ...;
  // Labels: method
```

## Health Endpoints

**HTTP Server (port 8080):**

### GET /health
Returns service health:
```json
{
  "status": "healthy",
  "service": "billing-service",
  "version": "1.0.0"
}
```

### GET /ready
Returns readiness (checks database connectivity):
```json
{
  "status": "ready",
  "checks": {
    "database": "ok",
    "invoicing_service": "ok"
  }
}
```
Returns 503 if any check fails.

### GET /metrics
Prometheus metrics endpoint.

## gRPC Health Check

Implements `grpc.health.v1.Health` service for load balancer probing:
- `SERVING` when ready to accept requests
- `NOT_SERVING` when database unavailable

## Grafana Dashboard

Dashboard includes:
- Request rate by method
- Error rate and types
- Request latency percentiles
- Billing run duration and success rate
- Subscriptions by status (gauge)
- Usage recording rate
- Invoice creation rate
- Database connection pool status

## Acceptance Criteria

- [ ] Logs are JSON formatted with trace context
- [ ] Traces appear in Tempo with full span hierarchy
- [ ] Traces propagate to invoicing-service calls
- [ ] Prometheus metrics available at /metrics
- [ ] Health endpoint returns service status
- [ ] Ready endpoint checks database connectivity
- [ ] gRPC health check returns SERVING when ready
- [ ] Grafana dashboard shows billing metrics

## Integration Tests

- [ ] Health endpoint returns 200 when healthy
- [ ] Ready endpoint returns 200 when database connected
- [ ] Ready endpoint returns 503 when database unavailable
- [ ] Metrics endpoint returns Prometheus format
- [ ] gRPC health check returns SERVING status
