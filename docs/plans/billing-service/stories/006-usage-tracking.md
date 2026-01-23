# Story: Usage Tracking

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement RecordUsage, GetUsage, and ListUsage gRPC methods for tracking usage-based billing components with idempotency support.

## Tasks

- [ ] Define proto messages: UsageRecord
- [ ] Define proto messages: RecordUsageRequest/Response
- [ ] Define proto messages: GetUsageRequest/Response
- [ ] Define proto messages: ListUsageRequest/Response
- [ ] Define proto messages: GetUsageSummaryRequest/Response
- [ ] Implement RecordUsage handler with idempotency
- [ ] Implement GetUsage handler
- [ ] Implement ListUsage handler with filters and pagination
- [ ] Implement GetUsageSummary handler for period aggregation
- [ ] Add capability checks to all methods
- [ ] Add metering for usage operations

## gRPC Methods

### RecordUsage
**Input:** tenant_id, subscription_id, component_id, quantity, timestamp, idempotency_key, metadata
**Output:** usage_record

**Validation:**
- subscription_id refers to active subscription
- component_id refers to valid usage_component on subscription's plan
- quantity > 0
- idempotency_key is unique (returns existing record if duplicate)
- timestamp is within current or previous billing cycle

**Business Logic:**
- Creates usage_record linked to current billing cycle
- Deduplicates based on idempotency_key (returns existing record)
- Assigns to appropriate billing cycle based on timestamp

**Capability:** `billing.usage:write`

### GetUsage
**Input:** tenant_id, record_id
**Output:** usage_record

**Capability:** `billing.usage:read`

### ListUsage
**Input:** tenant_id, subscription_id, component_id (optional), cycle_id (optional), is_invoiced (optional), page_size, page_token
**Output:** usage_records[], next_page_token

**Filters:**
- By subscription (required)
- By component (optional)
- By billing cycle (optional)
- By invoiced status (optional)

**Capability:** `billing.usage:read`

### GetUsageSummary
**Input:** tenant_id, subscription_id, cycle_id (optional, defaults to current)
**Output:** component_summaries[] (component_id, name, total_quantity, included_units, billable_units, amount)

**Business Logic:**
- Aggregates usage records by component for the period
- Calculates billable units = total - included
- Calculates amount = billable_units × unit_price

**Capability:** `billing.usage:read`

## Idempotency

The idempotency_key ensures exactly-once processing:
1. Before insert, check if key exists
2. If exists, return existing record (no error)
3. If not, insert new record with key
4. Key stored in usage_records table with UNIQUE constraint

Example:
```
# First request
RecordUsage(key="api-call-12345", quantity=100) → creates record

# Retry with same key
RecordUsage(key="api-call-12345", quantity=100) → returns same record (no duplicate)
```

## Metering

Record on each operation:
```rust
record_usage_operation(&tenant_id, "recorded");
record_usage_operation(&tenant_id, "queried");
record_usage_summary(&tenant_id);
```

## Acceptance Criteria

- [ ] RecordUsage creates usage record
- [ ] RecordUsage with duplicate key returns existing record
- [ ] RecordUsage validates subscription exists
- [ ] RecordUsage validates component belongs to plan
- [ ] RecordUsage assigns correct billing cycle
- [ ] GetUsage returns usage record
- [ ] GetUsage returns NOT_FOUND for missing record
- [ ] ListUsage filters by subscription, component, cycle
- [ ] ListUsage pagination works correctly
- [ ] GetUsageSummary aggregates by component
- [ ] GetUsageSummary calculates billable units correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Record usage with valid data returns record
- [ ] Record usage with idempotency key deduplicates
- [ ] Record usage with invalid subscription returns NOT_FOUND
- [ ] Record usage with invalid component returns NOT_FOUND
- [ ] Get usage returns complete record
- [ ] List usage by subscription returns matching records
- [ ] List usage by component filters correctly
- [ ] List usage by cycle filters correctly
- [ ] List usage pagination works
- [ ] Get usage summary calculates totals correctly
- [ ] Get usage summary handles included units
- [ ] Operations without capability return PERMISSION_DENIED
