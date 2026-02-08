# Epic 002: Direct & Offline Payments

## Summary

Enable tenants to record payments received outside of payment gateways — direct UPI transfers to bank accounts and offline payments (cash, cheque, bank transfer). Extends the Transaction model with payment channel and method type tracking.

## Problem

All payment flows currently require Razorpay gateway integration, incurring ~2% processing fees. Tenants receiving payments directly via UPI or offline methods have no way to record these in the system.

## Goals

- Extend Transaction model with `payment_channel`, `payment_method_type`, `external_reference`, and `notes`
- Add `RecordDirectUpiPayment` RPC with UTR-based deduplication
- Add `RecordOfflinePayment` RPC supporting cash, cheque, bank transfer, and other methods
- Maintain backward compatibility with existing Razorpay-based transactions

## Stories

| Story | Title | Status |
|-------|-------|--------|
| 012 | Transaction Model Extension | Planned |
| 013 | Direct UPI Payment Recording | Planned |
| 014 | Offline Payment Recording | Planned |

## Non-Goals

- Payment reconciliation (separate service)
- Auto-detection of UPI payments via bank APIs
- Approval workflows for offline payments
