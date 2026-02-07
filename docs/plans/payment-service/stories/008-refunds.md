# Story: Refunds

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement InitiateRefund, GetRefund, and ListRefunds gRPC methods for processing full and partial refunds with automatic transfer reversal support.

## Tasks

- [ ] Define proto messages: Refund, RefundStatus enum, RefundSpeed enum
- [ ] Define proto messages: InitiateRefundRequest/Response
- [ ] Define proto messages: GetRefundRequest/Response
- [ ] Define proto messages: ListRefundsRequest/Response
- [ ] Add Refund MongoDB collection and indexes
- [ ] Implement Razorpay refund API client
- [ ] Implement InitiateRefund handler with Razorpay integration
- [ ] Implement transfer reversal logic within refund flow
- [ ] Implement GetRefund handler
- [ ] Implement ListRefunds handler with filters and pagination
- [ ] Update transaction status to REFUNDED/PARTIALLY_REFUNDED on refund
- [ ] Add capability checks to all methods
- [ ] Add metering for refund operations

## gRPC Methods

### InitiateRefund
**Input:** tenant_id, payment_id, amount (optional — defaults to full refund), speed (normal, optimum), reverse_all_transfers, notes
**Output:** refund

**Validation:**
- Payment/transaction exists and is in COMPLETED or PARTIALLY_REFUNDED status
- Amount does not exceed captured amount minus prior refunds
- Amount is positive if specified
- Speed is valid enum

**Business Logic:**
- If amount omitted, refund full remaining amount
- If reverse_all_transfers = true, reverse all transfers associated with the payment
- Submit refund to Razorpay via Refunds API
- Store refund record with status CREATED
- Update transaction status:
  - Full refund → REFUNDED
  - Partial refund → PARTIALLY_REFUNDED

**Capability:** `payment.refund:create`

### GetRefund
**Input:** tenant_id, refund_id
**Output:** refund with current status

**Capability:** `payment.refund:read`

### ListRefunds
**Input:** tenant_id, payment_id (optional), status (optional), page_size, page_token
**Output:** refunds[], next_page_token

**Capability:** `payment.refund:read`

## Refund Status

| Status | Description |
|--------|-------------|
| `CREATED` | Refund initiated |
| `PROCESSED` | Refund completed |
| `FAILED` | Refund failed |

## Refund Speed

| Speed | Description |
|-------|-------------|
| `NORMAL` | Standard processing (5-7 business days) |
| `OPTIMUM` | Instant refund (if eligible) |

## Business Rules

- Refund amount cannot exceed captured amount minus sum of prior refund amounts
- Full refund sets transaction status to REFUNDED
- Partial refund sets transaction status to PARTIALLY_REFUNDED
- When reverse_all_transfers is true, all transfers for the payment are reversed proportionally
- Transfer reversal debits linked account, credits platform
- Refund speed "optimum" may not be available for all payment methods

## Metering

Record on each operation:
```rust
record_refund(&tenant_id, &status.to_string(), &speed.to_string());
record_refund_amount(&tenant_id, &currency, amount);
```

## Acceptance Criteria

- [ ] InitiateRefund creates refund in Razorpay
- [ ] InitiateRefund stores razorpay_refund_id
- [ ] InitiateRefund supports full refund (amount omitted)
- [ ] InitiateRefund supports partial refund
- [ ] InitiateRefund validates amount does not exceed available
- [ ] InitiateRefund returns INVALID_ARGUMENT for excessive amount
- [ ] InitiateRefund returns FAILED_PRECONDITION for non-completed payment
- [ ] InitiateRefund reverses transfers when reverse_all_transfers is true
- [ ] InitiateRefund updates transaction to REFUNDED on full refund
- [ ] InitiateRefund updates transaction to PARTIALLY_REFUNDED on partial refund
- [ ] InitiateRefund supports normal and optimum speed
- [ ] GetRefund returns refund with current status
- [ ] GetRefund returns NOT_FOUND for missing refund
- [ ] ListRefunds filters by payment_id
- [ ] ListRefunds filters by status
- [ ] ListRefunds pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Initiate full refund creates refund in Razorpay
- [ ] Initiate full refund updates transaction to REFUNDED
- [ ] Initiate partial refund creates refund with specified amount
- [ ] Initiate partial refund updates transaction to PARTIALLY_REFUNDED
- [ ] Initiate refund with amount exceeding available returns INVALID_ARGUMENT
- [ ] Initiate refund on non-completed payment returns FAILED_PRECONDITION
- [ ] Initiate refund on already-refunded payment returns FAILED_PRECONDITION
- [ ] Initiate refund with reverse_all_transfers reverses transfers
- [ ] Initiate refund with normal speed succeeds
- [ ] Initiate refund with optimum speed succeeds
- [ ] Get refund returns complete refund
- [ ] Get refund returns NOT_FOUND for missing refund
- [ ] List refunds filters by payment
- [ ] List refunds filters by status
- [ ] List refunds pagination works
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_refund_with_transfer_reversal` — Verifies [journey 005](../../../user-journeys/payment-service/005-refund-processing.md) steps 1-7: refund initiated, transfers reversed, customer refunded, transaction updated
- [ ] `journey_partial_refund` — Verifies [journey 005](../../../user-journeys/payment-service/005-refund-processing.md) with partial amount: partial refund processed, transaction status PARTIALLY_REFUNDED, proportional transfer reversal
