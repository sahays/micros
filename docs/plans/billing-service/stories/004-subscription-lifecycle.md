# Story: Subscription Lifecycle

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement ActivateSubscription, PauseSubscription, ResumeSubscription, and CancelSubscription gRPC methods for managing subscription lifecycle states.

## Tasks

- [ ] Define proto messages: ActivateSubscriptionRequest/Response
- [ ] Define proto messages: PauseSubscriptionRequest/Response
- [ ] Define proto messages: ResumeSubscriptionRequest/Response
- [ ] Define proto messages: CancelSubscriptionRequest/Response
- [ ] Implement ActivateSubscription handler (trial → active)
- [ ] Implement PauseSubscription handler
- [ ] Implement ResumeSubscription handler
- [ ] Implement CancelSubscription handler
- [ ] Add capability checks to all methods
- [ ] Add metering for lifecycle changes

## gRPC Methods

### ActivateSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription

**Validation:**
- Subscription must be in "trial" status
- Called when trial converts to paid (or manually early)

**Business Logic:**
- Changes status from "trial" to "active"
- If called before trial_end_date, clears trial_end_date

**Capability:** `billing.subscription:manage`

### PauseSubscription
**Input:** tenant_id, subscription_id, reason (optional)
**Output:** subscription

**Validation:**
- Subscription must be "active"
- Cannot pause if already paused or cancelled

**Business Logic:**
- Changes status to "paused"
- Paused subscriptions skip billing runs
- Records pause timestamp in metadata

**Capability:** `billing.subscription:manage`

### ResumeSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription

**Validation:**
- Subscription must be "paused"

**Business Logic:**
- Changes status back to "active"
- Optionally adjusts billing dates (configurable)
- Records resume timestamp in metadata

**Capability:** `billing.subscription:manage`

### CancelSubscription
**Input:** tenant_id, subscription_id, cancel_at_period_end, reason (optional)
**Output:** subscription

**Validation:**
- Subscription must be "active", "trial", or "paused"
- Cannot cancel already cancelled subscription

**Business Logic:**
- If cancel_at_period_end = true:
  - Status remains current until period end
  - end_date set to current_period_end
  - Final billing cycle processed at period end
- If cancel_at_period_end = false:
  - Status changes to "cancelled" immediately
  - No further billing
  - May trigger prorated credit (future enhancement)

**Capability:** `billing.subscription:manage`

## State Transitions

```
    ┌──────────────────────────────────────────┐
    │                                          │
    ▼                                          │
  TRIAL ──────► ACTIVE ◄─────► PAUSED          │
    │              │               │           │
    │              │               │           │
    └──────┬───────┴───────┬───────┘           │
           │               │                   │
           ▼               ▼                   │
       CANCELLED ◄─────────┘                   │
           │                                   │
           └───────────► EXPIRED ◄─────────────┘
```

## Business Rules (from spec)

1. Cancelled subscriptions bill through current period end (if cancel_at_period_end)
2. Paused subscriptions skip billing runs until resumed
3. Trials convert to paid automatically unless cancelled

## Metering

Record on each operation:
```rust
record_subscription_lifecycle(&tenant_id, "activated");
record_subscription_lifecycle(&tenant_id, "paused");
record_subscription_lifecycle(&tenant_id, "resumed");
record_subscription_lifecycle(&tenant_id, "cancelled");
```

## Acceptance Criteria

- [ ] ActivateSubscription transitions trial to active
- [ ] ActivateSubscription fails for non-trial subscription
- [ ] PauseSubscription transitions active to paused
- [ ] PauseSubscription fails for cancelled subscription
- [ ] ResumeSubscription transitions paused to active
- [ ] ResumeSubscription fails for non-paused subscription
- [ ] CancelSubscription with cancel_at_period_end sets end_date
- [ ] CancelSubscription without cancel_at_period_end cancels immediately
- [ ] CancelSubscription fails for already cancelled subscription
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Activate trial subscription succeeds
- [ ] Activate active subscription returns FAILED_PRECONDITION
- [ ] Pause active subscription succeeds
- [ ] Pause paused subscription returns FAILED_PRECONDITION
- [ ] Resume paused subscription succeeds
- [ ] Resume active subscription returns FAILED_PRECONDITION
- [ ] Cancel with period_end sets end_date correctly
- [ ] Cancel immediately changes status to cancelled
- [ ] Cancel cancelled subscription returns FAILED_PRECONDITION
- [ ] State transitions maintain data integrity
- [ ] Operations without capability return PERMISSION_DENIED
