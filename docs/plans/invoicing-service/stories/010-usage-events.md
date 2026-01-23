# Story: Usage Events

- [ ] **Status: Planning**
- **Epic:** [002-metering](../epics/002-metering.md)

## Summary

Implement RecordUsage, RecordUsageBatch, GetUsageEvent, and ListUsageEvents gRPC methods for usage event ingestion and retrieval.

## Tasks

- [ ] Create database migration for usage_events table
- [ ] Define proto messages: UsageEvent, RecordUsageRequest/Response
- [ ] Define proto messages: RecordUsageBatchRequest/Response
- [ ] Define proto messages: GetUsageEventRequest/Response
- [ ] Define proto messages: ListUsageEventsRequest/Response
- [ ] Implement RecordUsage handler with idempotency
- [ ] Implement RecordUsageBatch handler
- [ ] Implement GetUsageEvent handler
- [ ] Implement ListUsageEvents handler with filters and pagination
- [ ] Add metrics for usage ingestion rate

## Database Schema

```sql
CREATE TABLE usage_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    meter_id UUID NOT NULL REFERENCES meters(meter_id),
    customer_id UUID NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL,
    quantity DECIMAL(20, 8) NOT NULL,
    timestamp_utc TIMESTAMPTZ NOT NULL,
    properties JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, idempotency_key)
);

CREATE INDEX idx_usage_events_tenant_meter ON usage_events(tenant_id, meter_id);
CREATE INDEX idx_usage_events_tenant_customer ON usage_events(tenant_id, customer_id);
CREATE INDEX idx_usage_events_timestamp ON usage_events(tenant_id, timestamp_utc);
```

## gRPC Methods

### RecordUsage
**Input:** tenant_id, meter_id, customer_id, quantity, timestamp_utc, idempotency_key, properties
**Output:** usage_event

**Validation:**
- meter_id exists and belongs to tenant
- quantity > 0
- timestamp_utc not in future (with 5-minute tolerance)
- timestamp_utc not older than 30 days
- idempotency_key required and non-empty

**Idempotency:**
- If idempotency_key already exists for tenant, return existing event
- Prevents duplicate records from retries

### RecordUsageBatch
**Input:** tenant_id, events[] (each with meter_id, customer_id, quantity, timestamp_utc, idempotency_key, properties)
**Output:** results[] (event_id or error per input)

**Validation:**
- Max 1000 events per batch
- Same validation as RecordUsage per event
- Partial success allowed (some events may fail)

**Behavior:**
- Process all events, return success/failure per event
- Already-existing idempotency_keys return existing event (not error)

### GetUsageEvent
**Input:** tenant_id, event_id
**Output:** usage_event

### ListUsageEvents
**Input:** tenant_id, meter_id (optional), customer_id (optional), start_time, end_time, page_size, page_token
**Output:** events[], next_page_token

**Filters:**
- meter_id: Filter by specific meter
- customer_id: Filter by specific customer
- start_time/end_time: Filter by timestamp_utc range (required)

**Pagination:**
- Default page_size: 100, max: 1000
- Order by timestamp_utc DESC

## Acceptance Criteria

- [ ] RecordUsage creates usage event with valid data
- [ ] RecordUsage enforces idempotency (duplicate key returns same event)
- [ ] RecordUsage rejects invalid meter_id
- [ ] RecordUsage rejects future timestamps
- [ ] RecordUsage rejects timestamps older than 30 days
- [ ] RecordUsage rejects non-positive quantity
- [ ] RecordUsageBatch processes multiple events
- [ ] RecordUsageBatch handles partial failures
- [ ] RecordUsageBatch enforces max batch size
- [ ] GetUsageEvent returns event by ID
- [ ] GetUsageEvent returns NOT_FOUND for missing event
- [ ] ListUsageEvents filters by meter, customer, time range
- [ ] ListUsageEvents pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] Prometheus metrics track ingestion rate

## Integration Tests

- [ ] Record usage with valid data succeeds
- [ ] Record usage with duplicate idempotency_key returns same event
- [ ] Record usage with invalid meter returns NOT_FOUND
- [ ] Record usage with future timestamp returns INVALID_ARGUMENT
- [ ] Record usage with old timestamp returns INVALID_ARGUMENT
- [ ] Record usage with zero quantity returns INVALID_ARGUMENT
- [ ] Record batch with mixed valid/invalid returns partial results
- [ ] Record batch exceeding limit returns INVALID_ARGUMENT
- [ ] Get usage event returns complete event
- [ ] Get usage event wrong tenant returns NOT_FOUND
- [ ] List usage events filters correctly
- [ ] List usage events pagination works
