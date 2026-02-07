# User Journey: End Customer Payment

## Actor
End Customer (paying a tenant's product/service)

## Goal
Pay for a product or service offered by a tenant, with automatic fund splitting between the tenant and the platform.

## Services Involved
- payment-service
- ledger-service

## Razorpay Events
- `payment.captured`
- `transfer.processed`

## Flow

### Step 1: Create Order with Transfer Config
The tenant's application creates a Razorpay order via `CreateRazorpayOrder`, including transfer configuration that specifies the linked account and the split amounts. The commission is calculated based on the tenant's commission config.

### Step 2: Customer Completes Checkout
The end customer selects a payment method (card, UPI, netbanking, wallet) and completes the checkout via Razorpay's payment page or custom integration. Razorpay processes the payment.

### Step 3: Payment Captured
Payment-service receives the `payment.captured` webhook. The transaction record is updated to `COMPLETED` status with the Razorpay payment ID.

### Step 4: Transfers Auto-Created
Because the order was created with transfer configuration, Razorpay automatically creates transfers upon payment capture:
- Tenant share transferred to the linked account
- Platform retains the commission amount in its own balance

### Step 5: Transfer Processed
Payment-service receives `transfer.processed` webhook for each transfer. Transfer records are created/updated with status `PROCESSED`, recording the amount transferred to the linked account.

### Step 6: Records Updated
Transaction and transfer records are fully updated. The transaction is linked to the transfer(s). The payment amount, commission, and transfer amounts are all recorded for audit.

### Step 7: Settlement Scheduled
The transferred amount is scheduled for settlement to the tenant's bank account per the linked account's settlement schedule (default T+2).

## Commission Calculation

```
Payment Amount:     10,000 paise (100 INR)
Commission (5%):       500 paise (5 INR)
Transfer to Tenant:  9,500 paise (95 INR)
Platform Retains:      500 paise (5 INR)
```

## Error Scenarios

- **Payment failed:** `payment.failed` webhook received; transaction marked FAILED; no transfers created
- **Linked account not activated:** Order creation fails with FailedPrecondition; tenant must complete onboarding
- **Transfer failed:** `transfer.failed` webhook received; transfer marked FAILED; platform must investigate
