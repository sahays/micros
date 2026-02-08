# Story 013: Direct UPI Payment Recording

**Epic:** 002 — Direct & Offline Payments
**Status:** Planned
**Depends On:** Story 012

## Summary

Add `RecordDirectUpiPayment` RPC to record payments received directly via UPI with UTR-based deduplication.

## Changes

### Proto (`payment.proto`)
- Add `RecordDirectUpiPayment` RPC to PaymentService
- Add `RecordDirectUpiPaymentRequest`: amount_paise, currency, utr, payer_vpa (optional), notes (optional)
- Add `RecordDirectUpiPaymentResponse`: transaction

### Handler (`grpc/direct_payments.rs`)
- New module with `record_direct_upi_payment()` function
- Validates: amount > 0, UTR non-empty
- Checks duplicate UTR per tenant via `get_transaction_by_external_ref()`
- Creates Transaction in Completed status with channel=DirectUpi, method=Upi

### Capability
- `payment.direct_upi:record`

### Integration Tests
- Creates completed transaction with correct fields
- Rejects duplicate UTR
- Rejects zero amount
- Rejects empty UTR

## Acceptance Criteria
- Direct UPI payments recorded as Completed transactions
- Duplicate UTR detection prevents double-recording
- Transaction includes payment_channel=DirectUpi and payment_method_type=Upi
