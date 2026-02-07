# Story: Razorpay Subscription Orchestration

- [ ] **Status: Planning**
- **Epic:** [002-payment-service-integration](../epics/002-payment-service-integration.md)

## Summary

Implement billing-service orchestration of Razorpay subscriptions via payment-service. Billing runs create and manage Razorpay subscriptions, and billing-service reacts to subscription webhook events (charged, halted, cancelled) to update billing cycles and generate invoices.

## Tasks

- [ ] Add razorpay_plan_id and razorpay_subscription_id columns to billing tables
- [ ] Implement Razorpay plan creation during billing plan creation
- [ ] Implement Razorpay subscription creation during subscription enrollment
- [ ] Implement webhook event handler for subscription.charged
- [ ] Implement webhook event handler for subscription.halted
- [ ] Implement webhook event handler for subscription.cancelled
- [ ] Wire subscription pause to PauseRazorpaySubscription
- [ ] Wire subscription resume to ResumeRazorpaySubscription
- [ ] Wire subscription cancel to CancelRazorpaySubscription
- [ ] Add invoice creation on subscription.charged event
- [ ] Add notification on subscription.halted event
- [ ] Add metering for payment integration operations

## Orchestration Flow

### Plan Creation
When a billing plan is created, billing-service optionally creates a corresponding Razorpay plan:
1. Billing-service receives CreatePlan request
2. If Razorpay integration enabled, calls payment-service CreateRazorpayPlan
3. Stores razorpay_plan_id on the billing plan record

### Subscription Enrollment
When a customer subscribes to a plan:
1. Billing-service receives CreateSubscription request
2. Calls payment-service CreateCustomer (if not already created)
3. Calls payment-service CreateRazorpaySubscription with plan and customer
4. Stores razorpay_subscription_id on the billing subscription record
5. Returns short_url for customer authorization

### Charge Processing (subscription.charged)
When payment-service forwards subscription.charged webhook:
1. Find billing subscription by razorpay_subscription_id
2. Advance billing cycle to next period
3. Create charge records for the cycle
4. Create invoice via invoicing-service
5. Update billing cycle status to "invoiced"

### Subscription Halted (subscription.halted)
When payment-service forwards subscription.halted webhook:
1. Find billing subscription by razorpay_subscription_id
2. Update billing subscription status to reflect payment failure
3. Send notification via notification-service
4. Mark current billing cycle as "failed"

### Lifecycle Management
| Billing Action | Payment-Service Call |
|---------------|---------------------|
| Pause subscription | PauseRazorpaySubscription |
| Resume subscription | ResumeRazorpaySubscription |
| Cancel subscription | CancelRazorpaySubscription |

## Webhook Event Handling

Billing-service receives forwarded webhook events from payment-service (or via internal event bus):

| Event | Billing Action |
|-------|---------------|
| `subscription.charged` | Advance cycle, create invoice |
| `subscription.halted` | Mark subscription failed, notify |
| `subscription.cancelled` | Cancel billing subscription |
| `subscription.paused` | Pause billing subscription |
| `subscription.resumed` | Resume billing subscription |

## Metering

Record on each operation:
```rust
record_payment_integration(&tenant_id, "plan_created");
record_payment_integration(&tenant_id, "subscription_created");
record_payment_integration(&tenant_id, "charge_processed");
record_payment_integration(&tenant_id, "subscription_halted");
```

## Acceptance Criteria

- [ ] Billing plan creation creates Razorpay plan via payment-service
- [ ] Billing plan stores razorpay_plan_id
- [ ] Subscription enrollment creates Razorpay subscription via payment-service
- [ ] Subscription enrollment returns short_url for authorization
- [ ] Subscription enrollment stores razorpay_subscription_id
- [ ] subscription.charged event advances billing cycle
- [ ] subscription.charged event creates invoice via invoicing-service
- [ ] subscription.halted event marks subscription as failed
- [ ] subscription.halted event sends notification
- [ ] Billing pause calls PauseRazorpaySubscription
- [ ] Billing resume calls ResumeRazorpaySubscription
- [ ] Billing cancel calls CancelRazorpaySubscription
- [ ] All webhook handlers are idempotent
- [ ] Trace context propagated through all payment-service calls

## Integration Tests

- [ ] Create billing plan creates Razorpay plan via payment-service
- [ ] Create subscription creates Razorpay subscription via payment-service
- [ ] Create subscription returns short_url
- [ ] Charge webhook advances billing cycle
- [ ] Charge webhook creates invoice
- [ ] Halted webhook marks subscription as failed
- [ ] Pause subscription calls payment-service
- [ ] Resume subscription calls payment-service
- [ ] Cancel subscription calls payment-service
- [ ] Duplicate charge webhook is handled idempotently

## User Journey Integration Tests

- [ ] `journey_billing_creates_razorpay_subscription` — Verifies billing-service calls payment-service to create Razorpay subscription ([journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) step 1): billing plan created, Razorpay plan created, customer enrolled, subscription created with short_url
- [ ] `journey_billing_reacts_to_subscription_charged` — Verifies billing-service updates cycle on subscription.charged event ([journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) step 7): charge webhook received, billing cycle advanced, invoice created
- [ ] `journey_billing_handles_subscription_halted` — Verifies billing-service handles halted subscription ([journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) step 9): halted webhook received, subscription marked failed, notification sent
