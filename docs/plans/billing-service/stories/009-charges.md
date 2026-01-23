# Story: Charge Management

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement GetCharge, ListCharges, and CreateOneTimeCharge gRPC methods for managing individual billable items within billing cycles.

## Tasks

- [ ] Define proto messages: Charge, ChargeType enum
- [ ] Define proto messages: GetChargeRequest/Response
- [ ] Define proto messages: ListChargesRequest/Response
- [ ] Define proto messages: CreateOneTimeChargeRequest/Response
- [ ] Implement GetCharge handler
- [ ] Implement ListCharges handler with filters and pagination
- [ ] Implement CreateOneTimeCharge handler
- [ ] Add capability checks to all methods
- [ ] Add metering for charge operations

## gRPC Methods

### GetCharge
**Input:** tenant_id, charge_id
**Output:** charge

**Capability:** `billing.cycle:read`

### ListCharges
**Input:** tenant_id, cycle_id, charge_type (optional), page_size, page_token
**Output:** charges[], next_page_token

**Filters:**
- By billing cycle (required)
- By charge type (optional)

**Capability:** `billing.cycle:read`

### CreateOneTimeCharge
**Input:** tenant_id, subscription_id, description, amount, metadata
**Output:** charge

**Validation:**
- Subscription must be active
- Amount must be non-zero (positive for charge, negative for credit)

**Business Logic:**
- Creates charge in current pending billing cycle
- Type set to "one_time"
- Will be included in next billing run

**Capability:** `billing.charge:create`

## Charge Types

| Type | Description |
|------|-------------|
| `RECURRING` | Base plan price, created by billing run |
| `USAGE` | Usage-based charge, created by billing run |
| `ONE_TIME` | Ad-hoc charge or credit, created manually |
| `PRORATION` | Mid-cycle plan change adjustment |

## Charge Structure

```
charge {
  charge_id: UUID
  cycle_id: UUID
  charge_type: ChargeType
  description: "API Calls - 1,500 billable"
  quantity: Decimal (e.g., 1500)
  unit_price: Decimal (e.g., 0.001)
  amount: Decimal (e.g., 1.50)
  is_prorated: bool
  proration_factor: Decimal (optional, e.g., 0.5 for half period)
  component_id: UUID (optional, for usage charges)
  metadata: JSON
  created_utc: Timestamp
}
```

## Metering

Record on each operation:
```rust
record_charge_query(&tenant_id);
record_charge_created(&tenant_id, &charge_type);
```

## Acceptance Criteria

- [ ] GetCharge returns charge details
- [ ] GetCharge returns NOT_FOUND for missing charge
- [ ] ListCharges returns charges for cycle
- [ ] ListCharges filters by charge type
- [ ] ListCharges pagination works correctly
- [ ] CreateOneTimeCharge creates charge in current cycle
- [ ] CreateOneTimeCharge allows positive and negative amounts
- [ ] CreateOneTimeCharge fails for inactive subscription
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Get charge returns complete charge
- [ ] Get charge for missing ID returns NOT_FOUND
- [ ] List charges by cycle returns matching charges
- [ ] List charges by type filters correctly
- [ ] List charges pagination works
- [ ] Create one-time charge succeeds for active subscription
- [ ] Create one-time charge with negative amount (credit) succeeds
- [ ] Create one-time charge for inactive subscription returns FAILED_PRECONDITION
- [ ] Operations without capability return PERMISSION_DENIED
