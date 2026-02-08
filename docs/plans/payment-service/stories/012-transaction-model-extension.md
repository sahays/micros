# Story 012: Transaction Model Extension

**Epic:** 002 — Direct & Offline Payments
**Status:** Planned

## Summary

Extend the Transaction proto and Rust model to track how a payment was made (payment channel + method type), external references (UTR, cheque number), and notes.

## Changes

### Proto (`transaction.proto`)
- Add `PaymentChannel` enum: UNSPECIFIED, RAZORPAY, DIRECT_UPI, OFFLINE
- Add `PaymentMethodType` enum: UNSPECIFIED, UPI, CARD, NETBANKING, WALLET, CASH, CHEQUE, BANK_TRANSFER, OTHER
- Add 4 optional fields to Transaction message (fields 14-17): `payment_channel`, `payment_method_type`, `external_reference`, `notes`

### Rust Model (`models/mod.rs`)
- Add `PaymentChannel` enum with Serialize/Deserialize
- Add `PaymentMethodType` enum with Serialize/Deserialize
- Extend `Transaction` struct with 4 new `Option<_>` fields with `#[serde(default)]`

### Helpers (`grpc/helpers.rs`)
- Add `payment_channel_to_proto()` / `proto_to_payment_channel()` converters
- Add `payment_method_type_to_proto()` / `proto_to_payment_method_type()` converters
- Update `transaction_to_proto()` with 4 new optional fields

### Repository (`services/repository.rs`)
- Add sparse index on `{ app_id: 1, org_id: 1, external_reference: 1 }`
- Add `get_transaction_by_external_ref()` method

### Existing Code
- Update all `Transaction { .. }` struct literals with `None` for new fields

## Acceptance Criteria
- Proto compiles with new enums and fields
- Existing transactions deserialize correctly (new fields default to None)
- Sparse index created on external_reference
