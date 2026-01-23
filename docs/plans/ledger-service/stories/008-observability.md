# Story: Observability

- [ ] **Status: Planning**
- **Epic:** [001-ledger-service](../epics/001-ledger-service.md)

## Summary

Add Prometheus metrics, OpenTelemetry tracing, and structured logging for PLG stack compliance.

## Tasks

- [ ] Create metrics module with Prometheus registry
- [ ] Add gRPC request metrics (count, duration, status)
- [ ] Add transaction metrics (count, amount, entries per transaction)
- [ ] Add database operation metrics
- [ ] Add #[tracing::instrument] to all handlers and database functions
- [ ] Add structured logging with tenant_id, journal_id context
- [ ] Configure OTLP exporter for Tempo

## Metrics

| Metric | Type | Labels |
|--------|------|--------|
| ledger_grpc_requests_total | Counter | method, status |
| ledger_grpc_request_duration_seconds | Histogram | method |
| ledger_transactions_total | Counter | tenant_id, status |
| ledger_entries_total | Counter | direction |
| ledger_transaction_amount_total | Counter | currency |
| ledger_db_operations_total | Counter | operation, table |
| ledger_db_operation_duration_seconds | Histogram | operation |

## Tracing Spans

| Operation | Span Fields |
|-----------|-------------|
| PostTransaction | tenant_id, journal_id, entry_count, total_amount |
| GetBalance | tenant_id, account_id, as_of_date |
| GetStatement | tenant_id, account_id, date_range |
| DB queries | operation, table, row_count |

## Structured Logging

- All logs in JSON format
- Include: level, msg, timestamp, trace_id, span_id
- Include context: tenant_id, journal_id, account_id where applicable
- Error logs include error type and message

## Acceptance Criteria

- [ ] /metrics endpoint returns Prometheus format
- [ ] All gRPC methods have request count and duration metrics
- [ ] Transaction metrics track volume by tenant
- [ ] All handlers have tracing spans with relevant fields
- [ ] Logs are JSON formatted with trace correlation
- [ ] Traces exported to Tempo via OTLP

## Integration Tests

- [ ] Metrics endpoint returns valid Prometheus output
- [ ] Request increments grpc_requests_total counter
- [ ] Transaction increments transactions_total counter
