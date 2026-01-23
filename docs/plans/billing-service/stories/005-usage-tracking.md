# Story: Usage Tracking

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement RecordUsage and GetUsage gRPC methods for metered billing based on consumption.

## Tasks

- [ ] Define proto messages: UsageRecord, RecordUsageRequest/Response
- [ ] Define proto messages: GetUsageRequest/Response
- [ ] Implement RecordUsage handler with idempotency
- [ ] Implement GetUsage handler with aggregation
- [ ] Implement usage aggregation per billing cycle
- [ ] Handle out-of-order usage events

## gRPC Methods

### RecordUsage
**Input:** tenant_id, subscription_id, component_id, quantity, timestamp, idempotency_key
**Output:** usage_record

**Validation:**
- subscription_id exists and is active
- component_id exists on subscription's plan
- quantity > 0
- timestamp within reasonable range (not future, not too old)

**Idempotency:**
- If idempotency_key already exists, return existing record
- Prevents duplicate charges from retries

### GetUsage
**Input:** tenant_id, subscription_id, component_id (optional), period_start, period_end
**Output:** usage_summary (total_quantity, record_count, records[])

**Aggregation:**
- Sum quantity for component within period
- Return individual records if requested
- Group by component if component_id not specified

## Usage Aggregation for Billing

At billing time:
1. Sum usage per component for billing period
2. Subtract included_units from plan
3. Multiply overage by unit_price
4. Add to invoice as line items

Example:
- Plan includes 1000 API calls
- Recorded usage: 1500 calls
- Overage: 500 calls × $0.001 = $0.50

## Acceptance Criteria

- [ ] RecordUsage creates usage record
- [ ] RecordUsage enforces idempotency
- [ ] RecordUsage rejects invalid subscription/component
- [ ] RecordUsage rejects future timestamps
- [ ] GetUsage returns aggregated total
- [ ] GetUsage filters by component and period
- [ ] Usage correctly aggregated at billing time

## Integration Tests

- [ ] Record usage for valid subscription succeeds
- [ ] Record duplicate idempotency_key returns same record
- [ ] Record usage for cancelled subscription fails
- [ ] Get usage returns correct totals
- [ ] Get usage for empty period returns zero
