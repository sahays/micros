# Story: Subscription Lifecycle

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement ActivateSubscription, PauseSubscription, ResumeSubscription, CancelSubscription, and ChangePlan gRPC methods.

## Tasks

- [ ] Define proto messages for lifecycle operations
- [ ] Implement ActivateSubscription handler
- [ ] Implement PauseSubscription handler
- [ ] Implement ResumeSubscription handler
- [ ] Implement CancelSubscription handler
- [ ] Implement ChangePlan handler with proration flag
- [ ] Implement status transition validation

## gRPC Methods

### ActivateSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription

**Behavior:** Transition from paused to active. Resumes billing.

### PauseSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription

**Behavior:** Transition from active to paused. Skips billing runs until resumed.

### ResumeSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription

**Behavior:** Same as Activate. Recalculates billing period from resume date.

### CancelSubscription
**Input:** tenant_id, subscription_id, cancel_at_period_end (boolean)
**Output:** subscription

**Behavior:**
- If cancel_at_period_end = true: Mark for cancellation, continue until period end
- If cancel_at_period_end = false: Cancel immediately, prorate final invoice

### ChangePlan
**Input:** tenant_id, subscription_id, new_plan_id, prorate (boolean)
**Output:** subscription, proration_invoice_id (if applicable)

**Behavior:**
- Switch subscription to new plan
- If prorate = true: Calculate credit for unused time, charge for new plan
- If prorate = false: Change takes effect at next billing period

## Status Transitions

```
          ┌─────────┐
          │  active │◄─────────────────┐
          └────┬────┘                  │
               │                       │
    pause      │         resume        │
               ▼                       │
          ┌─────────┐                  │
          │ paused  │──────────────────┘
          └────┬────┘
               │
    cancel     │
               ▼
          ┌──────────┐
          │cancelled │
          └──────────┘
```

## Acceptance Criteria

- [ ] Pause stops subscription from billing runs
- [ ] Resume reactivates subscription
- [ ] Cancel at period end marks for future cancellation
- [ ] Cancel immediately cancels and prorates
- [ ] ChangePlan switches to new plan
- [ ] ChangePlan with proration creates adjustment invoice
- [ ] Invalid transitions return FAILED_PRECONDITION

## Integration Tests

- [ ] Pause active subscription succeeds
- [ ] Pause cancelled subscription fails
- [ ] Resume paused subscription succeeds
- [ ] Cancel at period end sets cancelled_at
- [ ] Cancel immediately changes status
- [ ] Change plan with proration creates invoice
