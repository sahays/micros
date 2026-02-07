# User Journey: Tenant Onboarding

## Actor
Platform Admin

## Goal
Onboard a new tenant as a payment-accepting merchant on the platform via Razorpay Route linked account.

## Services Involved
- auth-service
- payment-service
- notification-service

## Razorpay Events
- `account.under_review`
- `account.needs_clarification`
- `account.activated`

## Flow

### Step 1: Create Organization
Platform admin creates a new organization in auth-service for the tenant. This establishes the org_id that will be used across all services.

### Step 2: Submit Business Details
Admin submits the tenant's business details to payment-service via `CreateLinkedAccount`:
- Business name and type
- Legal information (PAN, GST)
- Bank account details for settlements
- Contact information

### Step 3: Create Razorpay Linked Account
Payment-service calls Razorpay Route API to create a linked account using platform credentials. The linked account is stored with status `CREATED` and the returned `razorpay_account_id`.

### Step 4: KYC Review Begins
Razorpay begins KYC verification of the tenant's business details. Payment-service receives `account.under_review` webhook and updates the linked account status to `UNDER_REVIEW`.

### Step 5: Additional Documents (if needed)
If Razorpay requires additional documentation, payment-service receives `account.needs_clarification` webhook. The linked account status updates to `NEEDS_CLARIFICATION`. Notification-service alerts the platform admin. Admin uploads additional documents via `UpdateLinkedAccount`.

### Step 6: Account Activated
Once Razorpay completes KYC verification, payment-service receives `account.activated` webhook. The linked account status updates to `ACTIVATED`. The tenant can now accept payments.

### Step 7: Configure Commission
Admin configures the commission rate for the tenant via `UpdateCommissionConfig`. Commission can be percentage-based, fixed, or both. This determines the platform's take on every payment processed through this tenant.

### Step 8: Ready to Accept Payments
The tenant is fully onboarded and can accept payments from their customers. Payments will be automatically split between the tenant and the platform based on the commission configuration.

## State Transitions

```
CREATED → UNDER_REVIEW → ACTIVATED
                       → NEEDS_CLARIFICATION → UNDER_REVIEW → ACTIVATED
                                                             → SUSPENDED
```

## Error Scenarios

- **Invalid business details:** CreateLinkedAccount returns validation error; admin corrects and resubmits
- **KYC rejection:** Razorpay may suspend the account; platform admin notified via webhook
- **Duplicate org_id:** CreateLinkedAccount returns AlreadyExists; one linked account per org
