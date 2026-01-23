# Story: Subscription Management

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement CreateSubscription, GetSubscription, and ListSubscriptions gRPC methods for managing customer subscriptions.

## Tasks

- [ ] Define proto messages: Subscription, CreateSubscriptionRequest/Response
- [ ] Define proto messages: GetSubscriptionRequest/Response, ListSubscriptionsRequest/Response
- [ ] Implement CreateSubscription handler
- [ ] Implement GetSubscription handler
- [ ] Implement ListSubscriptions handler with filters
- [ ] Implement billing period calculation
- [ ] Implement trial period handling

## gRPC Methods

### CreateSubscription
**Input:** tenant_id, customer_id, plan_id, billing_anchor (optional), trial_days (optional), metadata
**Output:** subscription

**Behavior:**
1. Validate plan exists and is active
2. Set billing_anchor (default: today)
3. Calculate current_period_start and current_period_end
4. Set trial_end if trial_days provided
5. Create subscription in active (or trialing) status

### GetSubscription
**Input:** tenant_id, subscription_id
**Output:** subscription with current plan details

### ListSubscriptions
**Input:** tenant_id, customer_id (optional), plan_id (optional), status (optional), page_size, page_token
**Output:** subscriptions[], next_page_token

## Billing Anchor

The billing anchor determines when billing periods start:
- Monthly plan with anchor on 15th: periods are 15th to 14th
- If anchor day > days in month, use last day of month
- Anchor set at subscription creation, persists through plan changes

## Trial Periods

- trial_end set to billing_anchor + trial_days
- During trial, status is active but no charges
- First invoice generated when trial ends
- Trial can be extended or ended early

## Acceptance Criteria

- [ ] CreateSubscription creates active subscription
- [ ] CreateSubscription calculates correct billing period
- [ ] CreateSubscription handles trial periods
- [ ] CreateSubscription rejects archived plans
- [ ] GetSubscription returns subscription with plan
- [ ] ListSubscriptions filters by customer, plan, status
- [ ] Billing periods align with anchor date

## Integration Tests

- [ ] Create subscription on active plan succeeds
- [ ] Create subscription on archived plan returns NOT_FOUND
- [ ] Create subscription with trial sets trial_end
- [ ] Billing period calculated correctly for monthly plan
- [ ] List subscriptions filters correctly
