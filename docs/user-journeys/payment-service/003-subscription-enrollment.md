# User Journey: Subscription Enrollment

## Actor
End Customer

## Goal
Subscribe to a tenant's recurring plan, authorize recurring charges, and receive ongoing service with automatic billing.

## Services Involved
- payment-service
- billing-service
- invoicing-service
- notification-service

## Razorpay Events
- `subscription.authenticated`
- `subscription.active`
- `subscription.charged`
- `subscription.pending`
- `subscription.halted`

## Flow

### Step 1: Create Razorpay Plan
The tenant creates a billing plan. Billing-service orchestrates payment-service to create the corresponding Razorpay plan via `CreateRazorpayPlan`, defining the period (monthly, yearly), interval, and amount.

### Step 2: Create Customer
The end customer's record is created in Razorpay via `CreateCustomer` with their name, email, and contact information. This links the internal user_id to a razorpay_customer_id.

### Step 3: Create Subscription
A subscription is created via `CreateRazorpaySubscription` linking the customer to the plan. Razorpay returns a `short_url` — a hosted page where the customer authorizes recurring charges.

### Step 4: Customer Authorizes
The customer visits the `short_url`, completes the first payment, and authorizes the payment method for future recurring charges. This is a mandatory step for recurring payments in India (RBI mandate).

### Step 5: Subscription Authenticated
Payment-service receives `subscription.authenticated` webhook. The subscription status updates to `AUTHENTICATED`, confirming the customer has authorized recurring charges.

### Step 6: Subscription Active
After the first charge is successfully processed, payment-service receives `subscription.active` webhook. The subscription status updates to `ACTIVE`. Recurring charges will now execute automatically on the billing cycle.

### Step 7: Recurring Charges
On each billing cycle, Razorpay automatically charges the customer. Payment-service receives `subscription.charged` webhook for each successful charge. The paid_count is incremented and remaining_count decremented. Billing-service updates the billing cycle and creates an invoice via invoicing-service.

### Step 8: Charge Failure and Retry
If a charge attempt fails, Razorpay retries automatically:
- T+1: First retry
- T+2: Second retry
- T+3: Third retry

Payment-service receives `subscription.pending` webhook during retry period. Notification-service sends payment failure alerts to the customer and tenant.

### Step 9: Subscription Halted
If all retry attempts fail, payment-service receives `subscription.halted` webhook. The subscription status updates to `HALTED`. No further automatic charges occur. Notification-service sends an alert requiring manual intervention — the tenant can share a new authorization link or the customer can update their payment method.

### Step 10: Pause, Resume, Cancel
The tenant can manage the subscription lifecycle through their application:
- **Pause:** `PauseRazorpaySubscription` — charges skipped until resumed
- **Resume:** `ResumeRazorpaySubscription` — charges resume from next cycle
- **Cancel:** `CancelRazorpaySubscription` — subscription ends (immediately or at cycle end)

## Subscription Lifecycle

```
CREATED → AUTHENTICATED → ACTIVE → COMPLETED
                              ↓
                          PENDING → ACTIVE (retry success)
                              ↓
                          HALTED (all retries failed)

ACTIVE → PAUSED → ACTIVE (resumed)
ACTIVE → CANCELLED
```

## Error Scenarios

- **Customer doesn't authorize:** Subscription remains in CREATED status; expires after timeout
- **Invalid payment method:** First charge fails; subscription moves to PENDING then HALTED if not resolved
- **Plan not found:** CreateRazorpaySubscription returns NotFound
- **Customer not found:** CreateRazorpaySubscription returns NotFound
