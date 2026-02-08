# Story 014: Offline Payment Recording

**Epic:** 002 — Direct & Offline Payments
**Status:** Planned
**Depends On:** Story 012

## Summary

Add `RecordOfflinePayment` RPC to record cash, cheque, bank transfer, and other manual payments.

## Changes

### Proto (`payment.proto`)
- Add `RecordOfflinePayment` RPC to PaymentService
- Add `RecordOfflinePaymentRequest`: amount_paise, currency, payment_method_type, external_reference (optional), notes (optional)
- Add `RecordOfflinePaymentResponse`: transaction

### Handler (`grpc/direct_payments.rs`)
- Add `record_offline_payment()` function to existing module
- Validates: amount > 0, valid payment_method_type (Cash, Cheque, BankTransfer, Other)
- Optional duplicate external_reference check per tenant
- Creates Transaction in Completed status with channel=Offline

### Capability
- `payment.offline:record`

### Integration Tests
- Records cash payment as completed transaction
- Records cheque payment with external reference
- Rejects duplicate external reference
- Rejects zero amount

## Acceptance Criteria
- Offline payments recorded as Completed transactions
- Supports Cash, Cheque, BankTransfer, Other method types
- Optional external reference for deduplication
- Transaction includes payment_channel=Offline
