# Story: Billing Cycles

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement GetBillingCycle, ListBillingCycles, and AdvanceBillingCycle gRPC methods for managing subscription billing periods.

## Tasks

- [ ] Define proto messages: BillingCycle, BillingCycleStatus enum
- [ ] Define proto messages: GetBillingCycleRequest/Response
- [ ] Define proto messages: ListBillingCyclesRequest/Response
- [ ] Define proto messages: AdvanceBillingCycleRequest/Response
- [ ] Implement GetBillingCycle handler
- [ ] Implement ListBillingCycles handler with filters and pagination
- [ ] Implement AdvanceBillingCycle handler
- [ ] Add capability checks to all methods
- [ ] Add metering for cycle operations

## gRPC Methods

### GetBillingCycle
**Input:** tenant_id, cycle_id
**Output:** billing_cycle with charges[]

**Includes:**
- Cycle details (period_start, period_end, status)
- All charges in the cycle
- Invoice reference if invoiced

**Capability:** `billing.cycle:read`

### ListBillingCycles
**Input:** tenant_id, subscription_id, status (optional), page_size, page_token
**Output:** billing_cycles[], next_page_token

**Filters:**
- By subscription (required)
- By status (optional)

**Capability:** `billing.cycle:read`

### AdvanceBillingCycle
**Input:** tenant_id, subscription_id
**Output:** previous_cycle, new_cycle

**Validation:**
- Subscription must be active
- Current cycle must be ready for advancement (typically after billing run)

**Business Logic:**
1. Closes current cycle (if not already invoiced)
2. Creates new cycle with next period dates
3. Updates subscription current_period_start/end
4. Returns both cycles

**Note:** Normally called by billing run, but exposed for manual advancement.

**Capability:** `billing.cycle:manage`

## Billing Cycle Status

| Status | Description |
|--------|-------------|
| `PENDING` | Active cycle, collecting charges and usage |
| `INVOICED` | Cycle closed, invoice generated |
| `PAID` | Invoice paid |
| `FAILED` | Invoice payment failed |

## Cycle Transitions

```
PENDING ──► INVOICED ──► PAID
                    └──► FAILED
```

## Metering

Record on each operation:
```rust
record_cycle_query(&tenant_id);
record_cycle_advanced(&tenant_id);
```

## Acceptance Criteria

- [ ] GetBillingCycle returns cycle with charges
- [ ] GetBillingCycle returns NOT_FOUND for missing cycle
- [ ] GetBillingCycle includes invoice reference if invoiced
- [ ] ListBillingCycles returns subscription's cycles
- [ ] ListBillingCycles filters by status
- [ ] ListBillingCycles pagination works correctly
- [ ] AdvanceBillingCycle creates new cycle
- [ ] AdvanceBillingCycle updates subscription periods
- [ ] AdvanceBillingCycle fails for inactive subscription
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Get billing cycle returns complete cycle
- [ ] Get billing cycle includes charges
- [ ] Get billing cycle includes invoice reference when invoiced
- [ ] List billing cycles by subscription returns matching cycles
- [ ] List billing cycles by status filters correctly
- [ ] List billing cycles pagination works
- [ ] Advance billing cycle creates new cycle
- [ ] Advance billing cycle closes previous cycle
- [ ] Advance billing cycle updates subscription dates
- [ ] Advance billing cycle for inactive subscription returns FAILED_PRECONDITION
- [ ] Operations without capability return PERMISSION_DENIED
