# Story: Subscription CRUD

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement CreateSubscription, GetSubscription, and ListSubscriptions gRPC methods for managing customer subscriptions to billing plans.

## Tasks

- [ ] Define proto messages: Subscription, SubscriptionStatus enum
- [ ] Define proto messages: CreateSubscriptionRequest/Response
- [ ] Define proto messages: GetSubscriptionRequest/Response
- [ ] Define proto messages: ListSubscriptionsRequest/Response
- [ ] Implement CreateSubscription handler with validation
- [ ] Implement GetSubscription handler
- [ ] Implement ListSubscriptions handler with filters and pagination
- [ ] Create initial billing cycle on subscription creation
- [ ] Add capability checks to all methods
- [ ] Add metering for subscription operations

## gRPC Methods

### CreateSubscription
**Input:** tenant_id, customer_id, plan_id, billing_anchor_day, start_date, trial_end_date, proration_mode, metadata
**Output:** subscription

**Validation:**
- plan_id refers to active, non-archived plan
- billing_anchor_day is 1-31
- start_date is not in the past (can be today or future)
- trial_end_date >= start_date if provided
- proration_mode is valid enum

**Business Logic:**
- Creates subscription in "active" status (or "trial" if trial_end_date set)
- Calculates current_period_start and current_period_end from billing_anchor_day
- Creates initial billing_cycle in "pending" status

**Capability:** `billing.subscription:create`

### GetSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription with current cycle info

**Capability:** `billing.subscription:read`

### ListSubscriptions
**Input:** tenant_id, customer_id (optional), status (optional), plan_id (optional), page_size, page_token
**Output:** subscriptions[], next_page_token

**Capability:** `billing.subscription:read`

## Subscription Status

| Status | Description |
|--------|-------------|
| `TRIAL` | In trial period, no billing |
| `ACTIVE` | Active and billing normally |
| `PAUSED` | Temporarily paused, skips billing |
| `CANCELLED` | Cancelled, bills through period end |
| `EXPIRED` | Past end_date, no longer active |

## Billing Anchor

The billing_anchor_day determines when billing cycles start:
- Day 1-28: Cycles start on that day each month
- Day 29-31: Cycles start on last day of month if month is shorter

Example: billing_anchor_day = 15
- Subscription starts Jan 20 → First cycle Jan 20 - Feb 14
- Next cycle Feb 15 - Mar 14
- Next cycle Mar 15 - Apr 14

## Metering

Record on each operation:
```rust
record_subscription(&tenant_id, "created");
record_subscription(&tenant_id, &status.to_string());
```

## Acceptance Criteria

- [ ] CreateSubscription returns new subscription
- [ ] CreateSubscription creates initial billing cycle
- [ ] CreateSubscription validates plan exists and is active
- [ ] CreateSubscription calculates correct period dates
- [ ] GetSubscription returns subscription with cycle info
- [ ] GetSubscription returns NOT_FOUND for missing subscription
- [ ] ListSubscriptions filters by customer, status, plan
- [ ] ListSubscriptions pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create subscription with valid data returns subscription
- [ ] Create subscription creates billing cycle
- [ ] Create subscription with trial sets trial_end_date
- [ ] Create subscription with archived plan returns FAILED_PRECONDITION
- [ ] Get subscription returns complete subscription
- [ ] Get subscription includes current cycle info
- [ ] List subscriptions returns only tenant's subscriptions
- [ ] List subscriptions with filters returns matching subset
- [ ] List subscriptions pagination works
- [ ] Operations without capability return PERMISSION_DENIED
