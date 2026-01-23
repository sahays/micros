# Story: Plan Changes

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement ChangePlan gRPC method for upgrading or downgrading subscriptions with proration support.

## Tasks

- [ ] Define proto messages: ChangePlanRequest/Response
- [ ] Define proto messages: ProrationMode enum
- [ ] Implement ChangePlan handler with proration calculation
- [ ] Implement immediate proration mode
- [ ] Implement next_cycle proration mode
- [ ] Implement none proration mode
- [ ] Create prorated charges for immediate mode
- [ ] Add capability checks
- [ ] Add metering for plan changes

## gRPC Methods

### ChangePlan
**Input:** tenant_id, subscription_id, new_plan_id, proration_mode (optional, uses subscription default)
**Output:** subscription, proration_charges[]

**Validation:**
- Subscription must be "active"
- new_plan_id must refer to active, non-archived plan
- new_plan must have same currency as current plan
- Cannot change to same plan

**Business Logic by proration_mode:**

**immediate:**
1. Calculate days remaining in current period
2. Calculate credit for unused portion of old plan
3. Calculate charge for remaining portion at new plan rate
4. Create prorated charges in current cycle
5. Update subscription to new plan immediately

**next_cycle:**
1. Record pending plan change
2. Plan change takes effect at next billing cycle
3. No proration charges

**none:**
1. Update subscription to new plan immediately
2. No proration calculations
3. Full new plan rate applies

**Capability:** `billing.subscription:change`

## Proration Calculation

Formula: `(days_used / days_in_period) * price`

Example (immediate mode):
- Old plan: $100/month, $3.33/day
- New plan: $150/month, $5.00/day
- Change on day 15 of 30-day month
- Days remaining: 15

Charges created:
1. Credit: -$50 (15 days × $3.33 unused old plan)
2. Charge: +$75 (15 days × $5.00 new plan)
- Net charge: +$25

## Metering

Record on each operation:
```rust
record_plan_change(&tenant_id, &old_plan_id, &new_plan_id);
record_proration(&tenant_id, &proration_mode);
```

## Acceptance Criteria

- [ ] ChangePlan with immediate mode creates proration charges
- [ ] ChangePlan with next_cycle mode schedules change
- [ ] ChangePlan with none mode applies immediately without proration
- [ ] ChangePlan validates new plan exists and is active
- [ ] ChangePlan validates currency matches
- [ ] ChangePlan fails for cancelled subscription
- [ ] Proration calculations are accurate
- [ ] Credit and charge amounts balance correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Change plan immediate mode creates credit and charge
- [ ] Change plan next_cycle mode schedules change
- [ ] Change plan none mode applies immediately
- [ ] Change to archived plan returns FAILED_PRECONDITION
- [ ] Change to plan with different currency returns INVALID_ARGUMENT
- [ ] Change cancelled subscription returns FAILED_PRECONDITION
- [ ] Proration amounts calculated correctly (upgrade)
- [ ] Proration amounts calculated correctly (downgrade)
- [ ] Operations without capability return PERMISSION_DENIED
