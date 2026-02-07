# User Journey: Payment Link Collection

## Actor
Tenant

## Goal
Send a payment link to collect a one-time or partial payment from a customer without requiring a checkout integration.

## Services Involved
- payment-service
- notification-service

## Razorpay Events
- `payment_link.paid`
- `payment_link.partially_paid`
- `payment_link.cancelled`
- `payment_link.expired`

## Flow

### Step 1: Create Payment Link
The tenant creates a payment link via `CreatePaymentLink`, specifying:
- Amount to collect
- Currency
- Description of the payment
- Optional: accept partial payments with minimum amount
- Optional: expiry timestamp

Payment-service creates the payment link in Razorpay and receives a `short_url` — a hosted payment page.

### Step 2: Share with Customer
The tenant shares the `short_url` with their customer through any channel:
- Email (via notification-service)
- SMS
- WhatsApp
- Any messaging platform

The customer does not need an account on the platform to pay.

### Step 3: Customer Opens Link
The customer opens the payment link in their browser. Razorpay displays a hosted payment page showing the amount, description, and available payment methods.

### Step 4: Customer Completes Payment
The customer selects a payment method (card, UPI, netbanking, wallet) and completes the payment.

### Step 5: Payment Link Paid
Payment-service receives `payment_link.paid` webhook. The payment link status updates to `PAID`. A transaction record is created and linked to the payment link.

### Step 6: Partial Payment (if enabled)
If `accept_partial` is true and the customer pays less than the full amount:
- Payment-service receives `payment_link.partially_paid` webhook
- Payment link status updates to `PARTIALLY_PAID`
- Customer can make additional payments until the full amount is collected
- Each partial payment creates a separate transaction linked to the payment link

### Step 7: Commission Split
If the tenant has a linked account configured, the commission split and transfer are created automatically, following the same flow as a regular end customer payment.

### Step 8: Transaction Created
A transaction record is created linking the payment to the payment link. The transaction includes the payment link ID for tracking and reconciliation.

### Step 9: Payment Link Expiry
If the payment link has an `expire_by` timestamp and expires before full payment:
- Payment-service receives `payment_link.expired` webhook
- Payment link status updates to `EXPIRED`
- No further payments are accepted on this link
- Any partial payments already received remain valid

## Payment Link Lifecycle

```
CREATED → PAID (full payment received)
CREATED → PARTIALLY_PAID → PAID (remaining paid)
CREATED → CANCELLED (by tenant)
CREATED → EXPIRED (past expire_by)
PARTIALLY_PAID → EXPIRED (past expire_by, partial amount retained)
```

## Error Scenarios

- **Invalid amount:** CreatePaymentLink returns InvalidArgument; amount must be positive
- **Min partial amount exceeds total:** Returns InvalidArgument; minimum partial must be less than total
- **Cancel after payment:** Returns FailedPrecondition; cannot cancel a PAID payment link
- **Payment on expired link:** Razorpay rejects the payment; customer sees expiry message
- **Duplicate payment link:** Each CreatePaymentLink creates a new link; no deduplication needed
