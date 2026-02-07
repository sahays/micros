# Story: Razorpay Plans

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreateRazorpayPlan, GetRazorpayPlan, and ListRazorpayPlans gRPC methods for managing Razorpay subscription plans.

## Tasks

- [ ] Define proto messages: RazorpayPlan, PlanPeriod enum
- [ ] Define proto messages: CreateRazorpayPlanRequest/Response
- [ ] Define proto messages: GetRazorpayPlanRequest/Response
- [ ] Define proto messages: ListRazorpayPlansRequest/Response
- [ ] Add RazorpayPlan MongoDB collection and indexes
- [ ] Implement Razorpay plan API client
- [ ] Implement CreateRazorpayPlan handler with Razorpay integration
- [ ] Implement GetRazorpayPlan handler
- [ ] Implement ListRazorpayPlans handler with pagination
- [ ] Add capability checks to all methods
- [ ] Add metering for plan operations
- [ ] Add RAZORPAY_SUBSCRIPTIONS_ENABLED feature flag check

## gRPC Methods

### CreateRazorpayPlan
**Input:** tenant_id, name, description, period (daily, weekly, monthly, yearly), interval, amount, currency
**Output:** plan

**Validation:**
- name is non-empty
- period is valid enum
- interval is positive
- amount is positive and in smallest currency unit
- currency is valid ISO code
- RAZORPAY_SUBSCRIPTIONS_ENABLED must be true

**Business Logic:**
- Creates plan in Razorpay via Subscriptions API
- Stores plan with razorpay_plan_id
- Plans are immutable once created in Razorpay

**Capability:** `payment.plan:create`

### GetRazorpayPlan
**Input:** tenant_id, plan_id
**Output:** plan

**Capability:** `payment.plan:read`

### ListRazorpayPlans
**Input:** tenant_id, page_size, page_token
**Output:** plans[], next_page_token

**Capability:** `payment.plan:read`

## Plan Periods

| Period | Description |
|--------|-------------|
| `DAILY` | Charge every N days |
| `WEEKLY` | Charge every N weeks |
| `MONTHLY` | Charge every N months |
| `YEARLY` | Charge every N years |

The `interval` field multiplies the period. For example: period=MONTHLY, interval=3 means charge every 3 months (quarterly).

## Metering

Record on each operation:
```rust
record_plan(&tenant_id, "created");
```

## Acceptance Criteria

- [ ] CreateRazorpayPlan creates plan in Razorpay
- [ ] CreateRazorpayPlan stores razorpay_plan_id
- [ ] CreateRazorpayPlan validates period and interval
- [ ] CreateRazorpayPlan validates amount is positive
- [ ] CreateRazorpayPlan checks RAZORPAY_SUBSCRIPTIONS_ENABLED flag
- [ ] GetRazorpayPlan returns plan details
- [ ] GetRazorpayPlan returns NOT_FOUND for missing plan
- [ ] ListRazorpayPlans returns only tenant's plans
- [ ] ListRazorpayPlans pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create plan with valid data returns plan with razorpay_plan_id
- [ ] Create plan with monthly period and interval 1 succeeds
- [ ] Create plan with yearly period and interval 1 succeeds
- [ ] Create plan with invalid period returns INVALID_ARGUMENT
- [ ] Create plan with zero amount returns INVALID_ARGUMENT
- [ ] Create plan with subscriptions disabled returns FAILED_PRECONDITION
- [ ] Get plan returns complete plan
- [ ] Get plan returns NOT_FOUND for missing plan
- [ ] List plans returns only tenant's plans
- [ ] List plans pagination works
- [ ] Operations without capability return PERMISSION_DENIED
