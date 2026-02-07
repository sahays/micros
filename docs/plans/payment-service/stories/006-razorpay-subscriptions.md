# Story: Razorpay Subscriptions

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreateRazorpaySubscription, GetRazorpaySubscription, ListRazorpaySubscriptions, PauseRazorpaySubscription, ResumeRazorpaySubscription, CancelRazorpaySubscription, and UpdateRazorpaySubscription gRPC methods for managing Razorpay subscription lifecycle.

## Tasks

- [ ] Define proto messages: RazorpaySubscription, SubscriptionStatus enum
- [ ] Define proto messages: CreateRazorpaySubscriptionRequest/Response
- [ ] Define proto messages: GetRazorpaySubscriptionRequest/Response
- [ ] Define proto messages: ListRazorpaySubscriptionsRequest/Response
- [ ] Define proto messages: PauseRazorpaySubscriptionRequest/Response
- [ ] Define proto messages: ResumeRazorpaySubscriptionRequest/Response
- [ ] Define proto messages: CancelRazorpaySubscriptionRequest/Response
- [ ] Define proto messages: UpdateRazorpaySubscriptionRequest/Response
- [ ] Add RazorpaySubscription MongoDB collection and indexes
- [ ] Implement Razorpay subscription API client
- [ ] Implement CreateRazorpaySubscription handler
- [ ] Implement GetRazorpaySubscription handler
- [ ] Implement ListRazorpaySubscriptions handler with filters and pagination
- [ ] Implement PauseRazorpaySubscription handler
- [ ] Implement ResumeRazorpaySubscription handler
- [ ] Implement CancelRazorpaySubscription handler
- [ ] Implement UpdateRazorpaySubscription handler
- [ ] Add capability checks to all methods
- [ ] Add metering for subscription operations
- [ ] Add RAZORPAY_SUBSCRIPTIONS_ENABLED feature flag check

## gRPC Methods

### CreateRazorpaySubscription
**Input:** tenant_id, plan_id, customer_id, total_count (optional), start_at (optional), expire_by (optional), customer_notify (optional), notes
**Output:** subscription with short_url

**Validation:**
- plan_id refers to existing Razorpay plan
- customer_id refers to existing Razorpay customer
- total_count is positive if provided
- start_at is in the future if provided
- RAZORPAY_SUBSCRIPTIONS_ENABLED must be true

**Business Logic:**
- Creates subscription in Razorpay via Subscriptions API
- Returns short_url for customer authorization
- Stores subscription with status CREATED
- Customer must visit short_url to authorize recurring charges

**Capability:** `payment.subscription:create`

### GetRazorpaySubscription
**Input:** tenant_id, subscription_id
**Output:** subscription with current status, cycle info, paid/remaining counts

**Capability:** `payment.subscription:read`

### ListRazorpaySubscriptions
**Input:** tenant_id, customer_id (optional), plan_id (optional), status (optional), page_size, page_token
**Output:** subscriptions[], next_page_token

**Capability:** `payment.subscription:read`

### PauseRazorpaySubscription
**Input:** tenant_id, subscription_id, pause_initiated_by (customer or plan)
**Output:** subscription with updated status

**Validation:**
- Subscription exists and is in ACTIVE status

**Business Logic:**
- Pauses subscription in Razorpay
- No charges collected while paused
- Updates local status to PAUSED

**Capability:** `payment.subscription:manage`

### ResumeRazorpaySubscription
**Input:** tenant_id, subscription_id, resume_at (optional)
**Output:** subscription with updated status

**Validation:**
- Subscription exists and is in PAUSED status

**Business Logic:**
- Resumes subscription in Razorpay
- Charges resume from next billing cycle
- Updates local status to ACTIVE

**Capability:** `payment.subscription:manage`

### CancelRazorpaySubscription
**Input:** tenant_id, subscription_id, cancel_at_cycle_end (boolean)
**Output:** subscription with updated status

**Validation:**
- Subscription exists and is in ACTIVE, PAUSED, or PENDING status

**Business Logic:**
- If cancel_at_cycle_end = true: subscription continues until current cycle ends, then cancels
- If cancel_at_cycle_end = false: subscription cancels immediately
- Updates local status to CANCELLED

**Capability:** `payment.subscription:manage`

### UpdateRazorpaySubscription
**Input:** tenant_id, subscription_id, plan_id (optional), total_count (optional), quantity (optional)
**Output:** subscription with updated parameters

**Validation:**
- Subscription exists and is in ACTIVE or AUTHENTICATED status
- New plan_id refers to existing plan if provided

**Business Logic:**
- Updates subscription parameters in Razorpay
- Plan changes take effect at next billing cycle
- Updates local record

**Capability:** `payment.subscription:manage`

## Subscription Status

| Status | Description |
|--------|-------------|
| `CREATED` | Awaiting customer authorization |
| `AUTHENTICATED` | Customer authorized, awaiting first charge |
| `ACTIVE` | First charge completed, recurring |
| `PENDING` | Charge failed, retrying (T+1, T+2, T+3) |
| `HALTED` | All retries exhausted |
| `PAUSED` | Paused by tenant |
| `CANCELLED` | Cancelled |
| `COMPLETED` | All planned charges completed |

## Metering

Record on each operation:
```rust
record_subscription(&tenant_id, "created");
record_subscription(&tenant_id, &status.to_string());
record_subscription_charge(&tenant_id, &charge_status);
```

## Acceptance Criteria

- [ ] CreateRazorpaySubscription creates subscription in Razorpay
- [ ] CreateRazorpaySubscription returns short_url for authorization
- [ ] CreateRazorpaySubscription validates plan and customer exist
- [ ] CreateRazorpaySubscription checks RAZORPAY_SUBSCRIPTIONS_ENABLED flag
- [ ] GetRazorpaySubscription returns subscription with cycle and count info
- [ ] GetRazorpaySubscription returns NOT_FOUND for missing subscription
- [ ] ListRazorpaySubscriptions filters by customer, plan, status
- [ ] ListRazorpaySubscriptions pagination works correctly
- [ ] PauseRazorpaySubscription pauses active subscription
- [ ] PauseRazorpaySubscription returns FAILED_PRECONDITION for non-active subscription
- [ ] ResumeRazorpaySubscription resumes paused subscription
- [ ] ResumeRazorpaySubscription returns FAILED_PRECONDITION for non-paused subscription
- [ ] CancelRazorpaySubscription cancels immediately
- [ ] CancelRazorpaySubscription cancels at cycle end
- [ ] UpdateRazorpaySubscription updates plan
- [ ] UpdateRazorpaySubscription updates total_count
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create subscription with valid plan and customer returns subscription
- [ ] Create subscription returns short_url
- [ ] Create subscription with missing plan returns NOT_FOUND
- [ ] Create subscription with missing customer returns NOT_FOUND
- [ ] Create subscription with subscriptions disabled returns FAILED_PRECONDITION
- [ ] Get subscription returns complete subscription with counts
- [ ] Get subscription returns NOT_FOUND for missing subscription
- [ ] List subscriptions filters by customer
- [ ] List subscriptions filters by plan
- [ ] List subscriptions filters by status
- [ ] List subscriptions pagination works
- [ ] Pause active subscription succeeds
- [ ] Pause non-active subscription returns FAILED_PRECONDITION
- [ ] Resume paused subscription succeeds
- [ ] Resume non-paused subscription returns FAILED_PRECONDITION
- [ ] Cancel subscription immediately succeeds
- [ ] Cancel subscription at cycle end succeeds
- [ ] Cancel non-cancellable subscription returns FAILED_PRECONDITION
- [ ] Update subscription plan succeeds
- [ ] Update subscription total_count succeeds
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_subscription_enrollment_full_flow` — Verifies [journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) steps 1-6: plan created, customer created, subscription created with short_url, authorization, activation
- [ ] `journey_subscription_charge_failure_retry_halt` — Verifies [journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) steps 8-9: charge fails, retries at T+1/T+2/T+3, subscription halted after all retries
