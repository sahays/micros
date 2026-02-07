# Story: Transfers

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreateTransferFromPayment, CreateTransferFromOrder, CreateDirectTransfer, ReverseTransfer, GetTransfer, and ListTransfers gRPC methods for managing Razorpay Route transfers between platform and linked accounts.

## Tasks

- [ ] Define proto messages: Transfer, TransferStatus enum
- [ ] Define proto messages: CreateTransferFromPaymentRequest/Response
- [ ] Define proto messages: CreateTransferFromOrderRequest/Response
- [ ] Define proto messages: CreateDirectTransferRequest/Response
- [ ] Define proto messages: ReverseTransferRequest/Response
- [ ] Define proto messages: GetTransferRequest/Response
- [ ] Define proto messages: ListTransfersRequest/Response
- [ ] Add Transfer MongoDB collection and indexes
- [ ] Implement Razorpay Route transfer API client
- [ ] Implement CreateTransferFromPayment handler
- [ ] Implement CreateTransferFromOrder handler
- [ ] Implement CreateDirectTransfer handler
- [ ] Implement ReverseTransfer handler
- [ ] Implement GetTransfer handler
- [ ] Implement ListTransfers handler with filters and pagination
- [ ] Implement commission calculation logic
- [ ] Add capability checks to all methods
- [ ] Add metering for transfer operations

## gRPC Methods

### CreateTransferFromPayment
**Input:** tenant_id, payment_id, linked_account_id, amount, currency, on_hold, on_hold_until
**Output:** transfer

**Validation:**
- Payment exists and is in COMPLETED status
- Linked account exists and is ACTIVATED
- Amount does not exceed remaining captured amount (captured - prior transfers)
- Amount is positive and in smallest currency unit

**Business Logic:**
- Calculates commission based on linked account's commission config
- Creates transfer via Razorpay Route API (POST /payments/{id}/transfers)
- Stores transfer record with razorpay_transfer_id
- Transfer amount = specified amount minus commission

**Capability:** `payment.transfer:create`

### CreateTransferFromOrder
**Input:** tenant_id, order_id, transfers[] (linked_account_id, amount, currency, on_hold, on_hold_until)
**Output:** transfers[]

**Validation:**
- Order exists
- All linked accounts exist and are ACTIVATED
- Total transfer amounts do not exceed order amount
- All amounts are positive

**Business Logic:**
- Creates order with transfer configuration in Razorpay
- Transfers auto-execute when payment is captured
- Stores transfer records

**Capability:** `payment.transfer:create`

### CreateDirectTransfer
**Input:** tenant_id, linked_account_id, amount, currency
**Output:** transfer

**Validation:**
- Linked account exists and is ACTIVATED
- Amount is positive
- Platform balance sufficient (checked by Razorpay)

**Business Logic:**
- Creates direct transfer from platform balance to linked account via Razorpay API
- Stores transfer record

**Capability:** `payment.transfer:create`

### ReverseTransfer
**Input:** tenant_id, transfer_id, amount (optional, defaults to full reversal)
**Output:** transfer with updated status

**Validation:**
- Transfer exists and is in PROCESSED status
- Reversal amount does not exceed transfer amount
- Amount is positive if specified

**Business Logic:**
- Reverses transfer via Razorpay Route API
- Updates transfer status to REVERSED
- Linked account balance debited, platform balance credited

**Capability:** `payment.transfer:reverse`

### GetTransfer
**Input:** tenant_id, transfer_id
**Output:** transfer

**Capability:** `payment.transfer:read`

### ListTransfers
**Input:** tenant_id, linked_account_id (optional), payment_id (optional), status (optional), page_size, page_token
**Output:** transfers[], next_page_token

**Capability:** `payment.transfer:read`

## Transfer Status

| Status | Description |
|--------|-------------|
| `CREATED` | Transfer created |
| `PENDING` | Transfer processing |
| `PROCESSED` | Transfer completed |
| `REVERSED` | Transfer reversed |
| `FAILED` | Transfer failed |

## Commission Calculation

```
Payment Amount:     10,000 paise
Commission (5%):       500 paise
Transfer Amount:     9,500 paise

With fixed commission (200 paise):
Payment Amount:     10,000 paise
Commission:            200 paise
Transfer Amount:     9,800 paise

With combined (3% + 100 paise):
Payment Amount:     10,000 paise
Commission:            400 paise (300 + 100)
Transfer Amount:     9,600 paise
```

## Metering

Record on each operation:
```rust
record_transfer(&tenant_id, &status.to_string());
record_transfer_amount(&tenant_id, &currency, amount);
record_commission_amount(&tenant_id, &currency, commission);
```

## Acceptance Criteria

- [ ] CreateTransferFromPayment creates transfer via Razorpay Route
- [ ] CreateTransferFromPayment calculates commission correctly
- [ ] CreateTransferFromPayment validates payment is COMPLETED
- [ ] CreateTransferFromPayment validates linked account is ACTIVATED
- [ ] CreateTransferFromPayment validates amount does not exceed captured
- [ ] CreateTransferFromOrder creates order-level transfers
- [ ] CreateTransferFromOrder auto-splits on payment capture
- [ ] CreateDirectTransfer transfers from platform balance
- [ ] CreateDirectTransfer validates linked account is ACTIVATED
- [ ] ReverseTransfer reverses processed transfer
- [ ] ReverseTransfer supports partial reversal
- [ ] ReverseTransfer returns FAILED_PRECONDITION for non-PROCESSED transfer
- [ ] GetTransfer returns transfer details
- [ ] GetTransfer returns NOT_FOUND for missing transfer
- [ ] ListTransfers filters by linked account, payment, status
- [ ] ListTransfers pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create transfer from payment returns transfer with razorpay_transfer_id
- [ ] Create transfer from payment calculates percentage commission
- [ ] Create transfer from payment calculates fixed commission
- [ ] Create transfer from payment calculates combined commission
- [ ] Create transfer from payment with non-completed payment returns FAILED_PRECONDITION
- [ ] Create transfer from payment with non-activated account returns FAILED_PRECONDITION
- [ ] Create transfer from payment exceeding captured amount returns INVALID_ARGUMENT
- [ ] Create transfer from order returns transfers
- [ ] Create direct transfer returns transfer
- [ ] Create direct transfer with non-activated account returns FAILED_PRECONDITION
- [ ] Reverse transfer reverses processed transfer
- [ ] Reverse transfer with partial amount reverses partial
- [ ] Reverse transfer on non-processed transfer returns FAILED_PRECONDITION
- [ ] Get transfer returns complete transfer
- [ ] List transfers filters by linked account
- [ ] List transfers filters by status
- [ ] List transfers pagination works
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_payment_splits_to_linked_account` — Verifies [journey 002](../../../user-journeys/payment-service/002-end-customer-payment.md) steps 1-6: order created with transfer config, payment captured, transfer processed to linked account
- [ ] `journey_payment_platform_retains_commission` — Verifies [journey 002](../../../user-journeys/payment-service/002-end-customer-payment.md): after transfer, platform balance increased by commission amount
