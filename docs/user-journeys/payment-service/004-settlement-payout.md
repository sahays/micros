# User Journey: Settlement Payout

## Actor
Tenant (linked account holder)

## Goal
Receive collected payment funds in the tenant's bank account through normal or on-demand settlement.

## Services Involved
- payment-service
- ledger-service

## Razorpay Events
- `settlement.processed`
- `transfer.processed`

## Flow

### Step 1: Payments Collected
Payments from end customers are captured and transfers are processed to the tenant's linked account. The transferred funds accumulate in the linked account's Razorpay balance.

### Step 2: Normal Settlement
By default, Razorpay settles the linked account balance to the tenant's bank account per the configured settlement schedule (T+2 is the default). This happens automatically without any API call.

### Step 3: Settlement Processed
Payment-service receives `settlement.processed` webhook with the settlement details including the UTR (Unique Transaction Reference) number. A settlement record is created with the amount, fees deducted by Razorpay, tax on fees, and the UTR for bank reconciliation.

### Step 4: Settlement Recorded
Payment-service records the settlement with full details:
- Settlement amount (after Razorpay fees and tax)
- Razorpay fees and tax deducted
- UTR number for bank tracking
- Linked account reference

### Step 5: On-Demand Settlement (Optional)
If the tenant needs funds urgently before the regular settlement schedule, the platform can request an instant or on-demand settlement via `RequestOnDemandSettlement`. This triggers immediate fund transfer to the tenant's bank account (subject to Razorpay's instant settlement availability and fees).

### Step 6: Settlement Hold (Platform-Initiated)
If the platform needs to hold funds for compliance, dispute resolution, or quality review, it can place a hold via `HoldTransferSettlement`:
- **Time-bound hold:** Funds released automatically after `on_hold_until` timestamp
- **Indefinite hold:** Funds held until manually released

### Step 7: Settlement Release
When the hold reason is resolved, the platform releases the hold via `ReleaseTransferSettlement`. The settlement proceeds according to the linked account's settlement schedule.

## Settlement Types

| Type | Description | Timing |
|------|-------------|--------|
| Normal | Automatic per schedule | T+2 (configurable) |
| Instant | On-demand immediate | Within minutes |
| On-demand | Requested by platform | Next settlement cycle |

## Settlement Calculation

```
Transfer Amount:     10,000 paise
Razorpay Fee (2%):      200 paise
Tax on Fee (18% GST):    36 paise
Settlement Amount:    9,764 paise
```

## Error Scenarios

- **Insufficient balance:** On-demand settlement fails if linked account balance is insufficient
- **Bank account invalid:** Settlement fails; Razorpay returns funds to linked account balance
- **Settlement on hold:** Normal settlement skipped while hold is active
- **Linked account suspended:** No settlements processed until account is reactivated
