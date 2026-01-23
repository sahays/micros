# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Add Prometheus metrics, OpenTelemetry tracing, and structured logging for production readiness.

## Tasks

- [ ] Add #[tracing::instrument] to all gRPC handlers
- [ ] Configure OpenTelemetry exporter to Tempo
- [ ] Add Prometheus metrics endpoint
- [ ] Define business metrics (subscriptions, billing runs, revenue)
- [ ] Configure structured JSON logging
- [ ] Add billing run status alerting

## Metrics

### Request Metrics
- `billing_grpc_requests_total{method, status}` - Request counter
- `billing_grpc_request_duration_seconds{method}` - Request latency histogram

### Business Metrics
- `billing_subscriptions_total{tenant_id, status}` - Subscription count by status
- `billing_subscriptions_mrr{tenant_id, currency}` - Monthly recurring revenue
- `billing_runs_total{tenant_id, status}` - Billing run count
- `billing_runs_duration_seconds{tenant_id}` - Billing run duration
- `billing_usage_records_total{tenant_id}` - Usage records count

### Alerting Metrics
- `billing_run_failures_total{tenant_id}` - Failed billing attempts
- `billing_subscription_churn_total{tenant_id}` - Cancellations

## Tracing

### Span Attributes
- tenant_id
- subscription_id / plan_id / run_id
- method name
- status code

### External Calls
- Trace invoicing-service gRPC calls
- Trace database queries
- Propagate trace context through billing runs

## Logging

### Log Events
- Subscription created/activated/cancelled
- Plan created/archived
- Usage recorded
- Billing run started/completed/failed
- Invoice created for subscription

### Log Fields
- timestamp, level, message
- service: billing-service
- trace_id, span_id
- tenant_id, subscription_id

## Acceptance Criteria

- [ ] All handlers have tracing spans
- [ ] Traces exported to Tempo
- [ ] Metrics endpoint returns Prometheus format
- [ ] MRR metric calculated correctly
- [ ] Billing run metrics track success/failure
- [ ] Logs include trace correlation
- [ ] Grafana dashboard created

## Integration Tests

- [ ] Metrics endpoint returns valid Prometheus format
- [ ] Trace IDs propagate through requests
- [ ] Billing run traces include all steps
