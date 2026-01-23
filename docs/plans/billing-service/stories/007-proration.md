# Story: Proration

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement proration logic for mid-cycle subscription changes including upgrades, downgrades, and cancellations.

## Tasks

- [ ] Implement proration calculation service
- [ ] Implement credit calculation for unused time
- [ ] Implement charge calculation for new plan
- [ ] Integrate proration with ChangePlan
- [ ] Integrate proration with immediate cancellation
- [ ] Create proration invoice/credit note

## Proration Modes

### Immediate Proration
- Calculate credit for unused portion of current plan
- Calculate charge for new plan for remaining period
- Create invoice with both (net charge or credit)
- Change takes effect immediately

### Next Cycle (No Proration)
- Current plan continues until period end
- New plan starts at next billing period
- No credits or additional charges

### None
- Change effective immediately
- No credits for unused time
- Full charge for new plan at next billing

## Calculation

**Credit for unused time:**
```
days_remaining = period_end - change_date
days_in_period = period_end - period_start
credit = (days_remaining / days_in_period) × old_plan_price
```

**Charge for new plan:**
```
days_remaining = period_end - change_date
days_in_period = period_end - period_start
charge = (days_remaining / days_in_period) × new_plan_price
```

**Net amount:**
```
net = charge - credit
if net > 0: create invoice for net amount
if net < 0: create credit note for |net| amount
```

## Examples

**Upgrade mid-month:**
- Day 15 of 30-day period
- Old plan: $30/month → Credit: $15 (15 days unused)
- New plan: $60/month → Charge: $30 (15 days remaining)
- Net invoice: $15

**Downgrade mid-month:**
- Day 15 of 30-day period
- Old plan: $60/month → Credit: $30 (15 days unused)
- New plan: $30/month → Charge: $15 (15 days remaining)
- Net credit: $15

## Acceptance Criteria

- [ ] Proration calculates correct credit for unused time
- [ ] Proration calculates correct charge for new plan
- [ ] Upgrade creates invoice for net charge
- [ ] Downgrade creates credit note for net credit
- [ ] Immediate cancellation prorates final period
- [ ] No proration mode changes at period end only
- [ ] Proration respects billing anchor

## Integration Tests

- [ ] Upgrade mid-cycle creates correct invoice
- [ ] Downgrade mid-cycle creates credit note
- [ ] Change at period boundary has no proration
- [ ] Cancellation prorates correctly
- [ ] Credit applied to next invoice
