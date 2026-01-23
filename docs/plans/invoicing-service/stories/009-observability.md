# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Add Prometheus metrics, OpenTelemetry tracing, and structured logging for production readiness.

## Tasks

- [ ] Add #[tracing::instrument] to all gRPC handlers
- [ ] Configure OpenTelemetry exporter to Tempo
- [ ] Add Prometheus metrics endpoint
- [ ] Define business metrics (invoice counts, payment volumes)
- [ ] Configure structured JSON logging
- [ ] Add request/response logging middleware
- [ ] Add error tracking with context

## Metrics

### Request Metrics
- `invoicing_grpc_requests_total{method, status}` - Request counter
- `invoicing_grpc_request_duration_seconds{method}` - Request latency histogram

### Business Metrics
- `invoicing_invoices_total{tenant_id, status}` - Invoice count by status
- `invoicing_invoices_amount_total{tenant_id, currency}` - Total invoiced amount
- `invoicing_payments_total{tenant_id, method}` - Payment count by method
- `invoicing_payments_amount_total{tenant_id, currency}` - Total payment amount

## Tracing

### Span Attributes
- tenant_id
- invoice_id / receipt_id / statement_id
- method name
- status code

### External Calls
- Trace ledger-service gRPC calls
- Trace database queries
- Propagate trace context

## Logging

### Log Fields
- timestamp
- level
- message
- service: invoicing-service
- trace_id, span_id
- tenant_id
- request_id

### Log Events
- Invoice created/issued/voided
- Payment recorded
- Statement generated
- Errors with full context

## Acceptance Criteria

- [ ] All handlers have tracing spans
- [ ] Traces exported to Tempo
- [ ] Metrics endpoint returns Prometheus format
- [ ] Business metrics track invoices and payments
- [ ] Logs include trace correlation
- [ ] Errors include stack context
- [ ] Grafana dashboard created

## Integration Tests

- [ ] Metrics endpoint returns valid Prometheus format
- [ ] Trace IDs propagate through requests
- [ ] Logs contain required fields
