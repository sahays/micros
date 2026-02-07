# User Journey: Refund Processing

## Actor
Tenant Admin (or automated refund policy)

## Goal
Refund an end customer's payment with automatic reversal of linked account transfers.

## Services Involved
- payment-service
- ledger-service

## Razorpay Events
- `refund.created`
- `refund.processed`
- `refund.failed`
- `transfer.reversed`

## Flow

### Step 1: Initiate Refund
The tenant initiates a refund via `InitiateRefund`, specifying:
- The payment/transaction to refund
- Refund amount (full or partial)
- Refund speed (normal or optimum)
- Whether to reverse all linked account transfers (`reverse_all_transfers`)

### Step 2: Validate Refund
Payment-service validates:
- The payment exists and is in COMPLETED status
- The refund amount does not exceed captured amount minus prior refunds
- The payment has not already been fully refunded

### Step 3: Transfer Reversal (if applicable)
If `reverse_all_transfers` is true and the payment had transfers to linked accounts:
- Razorpay reverses the transfers, debiting the linked account
- The reversed amount returns to the platform balance
- Payment-service receives `transfer.reversed` webhook for each reversal

### Step 4: Refund Submitted
Payment-service submits the refund to Razorpay. The refund is created in Razorpay and payment-service receives `refund.created` webhook. A refund record is stored with status `CREATED`.

### Step 5: Refund Processed
Razorpay processes the refund and credits the customer:
- **Normal speed:** 5-7 business days for the customer to receive funds
- **Optimum speed:** Instant refund to customer (if eligible)

Payment-service receives `refund.processed` webhook. The refund status updates to `PROCESSED`.

### Step 6: Transaction Status Updated
The transaction status is updated:
- Full refund: Status changes to `REFUNDED`
- Partial refund: Status changes to `PARTIALLY_REFUNDED`

### Step 7: Transfer Reversal Recorded
If transfers were reversed, the transfer records are updated to `REVERSED` status. The reversal amounts are recorded — the linked account balance is debited and the platform balance is credited.

### Step 8: Ledger Entries
Ledger-service records the reversal entries:
- Credit to customer (refund)
- Debit to linked account (transfer reversal)
- Credit to platform (commission reversal, if applicable)

## Refund Calculation (Partial)

```
Original Payment:     10,000 paise
Original Transfer:     9,500 paise (to tenant)
Original Commission:     500 paise (platform)

Partial Refund:        5,000 paise
Transfer Reversal:     4,750 paise (from tenant)
Commission Reversal:     250 paise (from platform)
Customer Receives:     5,000 paise
```

## Refund Speed

| Speed | Customer Impact | Availability |
|-------|----------------|--------------|
| Normal | 5-7 business days | Always available |
| Optimum | Instant (within seconds) | Subject to eligibility |

## Error Scenarios

- **Refund exceeds available amount:** Returns InvalidArgument; refund amount cannot exceed captured minus prior refunds
- **Payment not found:** Returns NotFound
- **Payment not in COMPLETED status:** Returns FailedPrecondition; can only refund completed payments
- **Refund failed at Razorpay:** `refund.failed` webhook; refund status updated to FAILED; may need retry
- **Linked account insufficient balance:** Transfer reversal may fail if tenant has already withdrawn funds
