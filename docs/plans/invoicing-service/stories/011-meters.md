# Story: Meters

- [ ] **Status: Planning**
- **Epic:** [002-metering](../epics/002-metering.md)

## Summary

Implement CreateMeter, GetMeter, ListMeters, and UpdateMeter gRPC methods for meter configuration management.

## Tasks

- [ ] Create database migration for meters table
- [ ] Define proto messages: Meter, AggregationType enum
- [ ] Define proto messages: CreateMeterRequest/Response
- [ ] Define proto messages: GetMeterRequest/Response
- [ ] Define proto messages: ListMetersRequest/Response
- [ ] Define proto messages: UpdateMeterRequest/Response
- [ ] Implement CreateMeter handler with validation
- [ ] Implement GetMeter handler
- [ ] Implement ListMeters handler with pagination
- [ ] Implement UpdateMeter handler

## Database Schema

```sql
CREATE TYPE aggregation_type AS ENUM ('sum', 'max', 'last', 'unique_count');

CREATE TABLE meters (
    meter_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    unit_name VARCHAR(50) NOT NULL,
    aggregation_type aggregation_type NOT NULL DEFAULT 'sum',
    metadata JSONB,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, name)
);

CREATE INDEX idx_meters_tenant ON meters(tenant_id);
CREATE INDEX idx_meters_tenant_active ON meters(tenant_id, is_active);
```

## gRPC Methods

### CreateMeter
**Input:** tenant_id, name, display_name, unit_name, aggregation_type, metadata
**Output:** meter

**Validation:**
- name: required, alphanumeric with underscores, max 100 chars
- display_name: required, max 255 chars
- unit_name: required, max 50 chars
- aggregation_type: must be valid enum value
- name must be unique per tenant

**Examples:**
```
name: "api_calls"
display_name: "API Calls"
unit_name: "call"
aggregation_type: sum
```

### GetMeter
**Input:** tenant_id, meter_id
**Output:** meter

### ListMeters
**Input:** tenant_id, include_inactive (default false), page_size, page_token
**Output:** meters[], next_page_token

**Filters:**
- include_inactive: If false, only return active meters

**Pagination:**
- Default page_size: 50, max: 200
- Order by name ASC

### UpdateMeter
**Input:** tenant_id, meter_id, display_name, unit_name, metadata, is_active
**Output:** meter

**Validation:**
- Cannot update name or aggregation_type (would invalidate existing events)
- Can deactivate meter (is_active = false)
- Deactivated meters reject new usage events

## Aggregation Types

| Type | Description | Use Case |
|------|-------------|----------|
| `sum` | Total of all quantities | API calls, bytes transferred, messages sent |
| `max` | Maximum value in period | Concurrent users, peak connections |
| `last` | Last recorded value | Current storage level, seat count |
| `unique_count` | Count of unique property values | Active users (unique user_id in properties) |

## Acceptance Criteria

- [ ] CreateMeter creates meter with valid data
- [ ] CreateMeter rejects duplicate name for tenant
- [ ] CreateMeter rejects invalid name format
- [ ] CreateMeter rejects invalid aggregation_type
- [ ] GetMeter returns meter by ID
- [ ] GetMeter returns NOT_FOUND for missing meter
- [ ] ListMeters returns only tenant's meters
- [ ] ListMeters filters inactive meters by default
- [ ] ListMeters includes inactive when requested
- [ ] ListMeters pagination works correctly
- [ ] UpdateMeter modifies allowed fields
- [ ] UpdateMeter rejects name change
- [ ] UpdateMeter rejects aggregation_type change
- [ ] UpdateMeter can deactivate meter
- [ ] All methods enforce tenant isolation

## Integration Tests

- [ ] Create meter with valid data succeeds
- [ ] Create meter with duplicate name returns ALREADY_EXISTS
- [ ] Create meter with invalid name returns INVALID_ARGUMENT
- [ ] Get meter returns complete meter
- [ ] Get meter wrong tenant returns NOT_FOUND
- [ ] List meters returns only active by default
- [ ] List meters with include_inactive returns all
- [ ] List meters pagination works
- [ ] Update meter display_name succeeds
- [ ] Update meter name returns INVALID_ARGUMENT
- [ ] Update meter aggregation_type returns INVALID_ARGUMENT
- [ ] Deactivate meter succeeds
- [ ] Record usage on inactive meter returns FAILED_PRECONDITION
