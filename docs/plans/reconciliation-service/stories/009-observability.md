# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Add Prometheus metrics, OpenTelemetry tracing, and structured logging for production readiness.

## Tasks

- [ ] Add #[tracing::instrument] to all gRPC handlers
- [ ] Configure OpenTelemetry exporter to Tempo
- [ ] Add Prometheus metrics endpoint
- [ ] Define business metrics (reconciliations, matches, AI accuracy)
- [ ] Configure structured JSON logging
- [ ] Add AI suggestion accuracy tracking

## Metrics

### Request Metrics
- `reconciliation_grpc_requests_total{method, status}` - Request counter
- `reconciliation_grpc_request_duration_seconds{method}` - Request latency histogram

### Business Metrics
- `reconciliation_statements_total{tenant_id, status}` - Statement count by status
- `reconciliation_transactions_total{tenant_id, status}` - Transaction count by match status
- `reconciliation_matches_total{tenant_id, type}` - Match count by type (auto, manual, ai)
- `reconciliation_sessions_total{tenant_id, status}` - Reconciliation session count
- `reconciliation_difference_amount{tenant_id}` - Current reconciliation difference

### AI Metrics
- `reconciliation_ai_suggestions_total{tenant_id}` - AI suggestions generated
- `reconciliation_ai_confirmed_total{tenant_id}` - AI suggestions confirmed
- `reconciliation_ai_rejected_total{tenant_id}` - AI suggestions rejected
- `reconciliation_ai_accuracy{tenant_id}` - Confirmation rate (confirmed / total)

## Tracing

### Span Attributes
- tenant_id
- bank_account_id / statement_id / reconciliation_id
- method name
- status code

### External Calls
- Trace ledger-service gRPC calls
- Trace genai-service calls (with timing)
- Trace document-service calls
- Trace database queries

## Logging

### Log Events
- Statement imported (transaction count)
- Match created (type, confidence)
- AI suggestion generated/confirmed/rejected
- Reconciliation started/completed/abandoned
- Adjustment created

### Log Fields
- timestamp, level, message
- service: reconciliation-service
- trace_id, span_id
- tenant_id, reconciliation_id

## Acceptance Criteria

- [ ] All handlers have tracing spans
- [ ] Traces exported to Tempo
- [ ] Metrics endpoint returns Prometheus format
- [ ] AI accuracy metrics tracked
- [ ] Match type distribution visible
- [ ] Logs include trace correlation
- [ ] Grafana dashboard created

## Integration Tests

- [ ] Metrics endpoint returns valid Prometheus format
- [ ] Trace IDs propagate through requests
- [ ] AI suggestion metrics increment correctly
