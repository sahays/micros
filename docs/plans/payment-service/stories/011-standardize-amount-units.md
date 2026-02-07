# Story: Standardize Amount Units to Paise

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Standardize all monetary amount fields across the payment-service API to use the smallest currency unit (paise for INR) as `uint64`. The legacy Transaction API uses `double` amounts in rupees while all newer APIs (transfers, refunds, settlements, payment links, subscriptions) use `uint64` paise. This inconsistency creates a bug-prone API surface where BFF clients can silently pass amounts in the wrong unit.

## Problem

The payment-service has two conflicting amount conventions:

| API | Type | Unit | Example for 100 INR |
|-----|------|------|---------------------|
| CreateTransaction | `double` | rupees | `100.0` |
| GenerateUpiQr | `double` | rupees | `100.0` |
| CreateTransferFromPayment | `uint64` | paise | `10000` |
| InitiateRefund | `uint64` | paise | `10000` |
| CreatePaymentLink | `uint64` | paise | `10000` |
| RequestOnDemandSettlement | `uint64` | paise | `10000` |
| CreateRazorpayPlan | `uint64` | paise | `10000` |

Additionally, there are 3 float↔int conversion points in handler code that risk precision bugs:
- `payment_service.rs:306` — `req.amount as f64 / 100.0`
- `payment_service.rs:122` — `(transaction.amount * 100.0) as u64`
- `refunds.rs:52` — `(payment.amount * 100.0) as u64`

## Tasks

- [ ] Change `Transaction.amount` and `CreateTransactionRequest.amount` from `double` to `uint64` in `transaction.proto`
- [ ] Change `GenerateUpiQrRequest.amount` from `double` to `uint64` in `payment.proto`
- [ ] Update all proto amount field comments to say "Amount in smallest currency unit (e.g., paise for INR)"
- [ ] Change `Transaction.amount` from `f64` to `u64` in Rust model
- [ ] Add `deserialize_amount` serde deserializer to handle both BSON Double (legacy) and Int64 (new) during migration
- [ ] Remove float conversion in `create_razorpay_order` handler (`req.amount as f64 / 100.0` → `req.amount`)
- [ ] Remove float conversion in `create_transaction` metrics (`(transaction.amount * 100.0) as u64` → `transaction.amount`)
- [ ] Remove float conversion in `initiate_refund` handler (`(payment.amount * 100.0) as u64` → `payment.amount`)
- [ ] Update `generate_upi_qr` handler to convert paise→rupees only at UPI link format boundary
- [ ] Update `UpiService::generate_upi_link` to accept `u64` paise and convert internally
- [ ] Add amount validation (> 0) in `create_transaction` and `create_razorpay_order`
- [ ] Update DTOs (`dtos/mod.rs`) amount fields from `f64` to `u64`
- [ ] Update HTTP handler (`handlers/razorpay.rs`) to remove float conversion
- [ ] Update `service-core` PaymentClient: `create_transaction` and `generate_upi_qr` amount params from `f64` to `u64`
- [ ] Update all payment-service integration tests to use paise values
- [ ] Update workflow-tests (`payment_ledger_test.rs`) to use paise values
- [ ] Create MongoDB migration script to convert existing transaction amounts from rupees to paise

## Files Modified

### Proto
- `proto/micros/payment/v1/transaction.proto` — `double` → `uint64` for amount fields
- `proto/micros/payment/v1/payment.proto` — `double` → `uint64` for `GenerateUpiQrRequest.amount`

### Models
- `payment-service/src/models/mod.rs` — `Transaction.amount`: `f64` → `u64` + migration deserializer

### gRPC Handlers
- `payment-service/src/grpc/payment_service.rs` — Remove 3 float conversions, add validation
- `payment-service/src/grpc/refunds.rs` — Remove float conversion

### Services
- `payment-service/src/services/upi.rs` — Accept `u64` paise, convert to rupees internally

### DTOs + HTTP Handlers
- `payment-service/src/dtos/mod.rs` — `f64` → `u64`
- `payment-service/src/handlers/razorpay.rs` — Remove conversion, update response type
- `payment-service/src/handlers/upi.rs` — Pass paise directly

### service-core
- `service-core/src/grpc/payment_client.rs` — `f64` → `u64` parameters

### Tests
- `payment-service/tests/payment_test.rs` — rupee values → paise values
- `payment-service/tests/refund_test.rs` — rupee values in helper → paise values
- `workflow-tests/tests/payment_ledger_test.rs` — rupee values → paise values

### Migration
- `scripts/migrate_transaction_amounts.js` — Convert existing MongoDB documents

## Migration Strategy

1. **Deploy with dual-format deserializer**: The custom `deserialize_amount` function reads both BSON Double (legacy rupees, converted to paise) and Int64 (new paise format). This allows the service to handle mixed documents.
2. **Run migration script**: `mongosh payment_db scripts/migrate_transaction_amounts.js` converts all existing transaction amounts from rupees (Double) to paise (Int64).
3. **Optional follow-up**: Remove custom deserializer, use plain `u64` field.

## Acceptance Criteria

- [ ] All proto amount fields use `uint64` with "smallest currency unit" comments
- [ ] Transaction model stores amounts as `u64` paise
- [ ] No floating-point arithmetic in amount handling (except UPI link formatting boundary)
- [ ] `create_transaction` rejects amount = 0
- [ ] `create_razorpay_order` rejects amount = 0
- [ ] UPI QR still generates correct `am=` parameter in rupees (e.g., `am=100.00` for 10000 paise)
- [ ] Refund available-amount calculation uses integer math only
- [ ] Metrics record amounts directly without float conversion
- [ ] Migration deserializer handles both BSON Double and Int64
- [ ] MongoDB migration script converts legacy documents

## Integration Tests

- [ ] Create transaction with paise amount, verify stored and returned correctly
- [ ] Create Razorpay order with paise amount, verify transaction amount matches
- [ ] Generate UPI QR with 10000 paise, verify `am=100.00` in UPI link
- [ ] Initiate refund on transaction created with paise, verify calculation correctness
- [ ] Partial refund amount validation works with paise values
- [ ] All existing tests pass with updated paise values
