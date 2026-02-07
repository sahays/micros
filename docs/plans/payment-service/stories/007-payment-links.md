# Story: Payment Links

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreatePaymentLink, GetPaymentLink, CancelPaymentLink, and ListPaymentLinks gRPC methods for managing Razorpay payment links for one-time and partial payment collection.

## Tasks

- [ ] Define proto messages: PaymentLink, PaymentLinkStatus enum
- [ ] Define proto messages: CreatePaymentLinkRequest/Response
- [ ] Define proto messages: GetPaymentLinkRequest/Response
- [ ] Define proto messages: CancelPaymentLinkRequest/Response
- [ ] Define proto messages: ListPaymentLinksRequest/Response
- [ ] Add PaymentLink MongoDB collection and indexes
- [ ] Implement Razorpay payment link API client
- [ ] Implement CreatePaymentLink handler with Razorpay integration
- [ ] Implement GetPaymentLink handler
- [ ] Implement CancelPaymentLink handler
- [ ] Implement ListPaymentLinks handler with filters and pagination
- [ ] Add capability checks to all methods
- [ ] Add metering for payment link operations

## gRPC Methods

### CreatePaymentLink
**Input:** tenant_id, amount, currency, description, accept_partial (optional), min_partial_amount (optional), expire_by (optional), customer_name (optional), customer_email (optional), customer_contact (optional), notes
**Output:** payment_link with short_url

**Validation:**
- amount is positive and in smallest currency unit
- currency is valid ISO code
- description is non-empty
- min_partial_amount < amount if accept_partial is true
- expire_by is in the future if provided

**Business Logic:**
- Creates payment link in Razorpay via Payment Links API
- Returns short_url for sharing with customer
- Stores payment link record with status CREATED

**Capability:** `payment.payment_link:create`

### GetPaymentLink
**Input:** tenant_id, payment_link_id
**Output:** payment_link with current status and payment details

**Capability:** `payment.payment_link:read`

### CancelPaymentLink
**Input:** tenant_id, payment_link_id
**Output:** payment_link with updated status

**Validation:**
- Payment link exists
- Payment link is in CREATED or PARTIALLY_PAID status (cannot cancel PAID, EXPIRED, or already CANCELLED)

**Business Logic:**
- Cancels payment link in Razorpay
- Updates local status to CANCELLED
- No further payments accepted on this link

**Capability:** `payment.payment_link:cancel`

### ListPaymentLinks
**Input:** tenant_id, status (optional), page_size, page_token
**Output:** payment_links[], next_page_token

**Capability:** `payment.payment_link:read`

## Payment Link Status

| Status | Description |
|--------|-------------|
| `CREATED` | Link created, awaiting payment |
| `PARTIALLY_PAID` | Partial payment received |
| `PAID` | Full payment received |
| `CANCELLED` | Cancelled by tenant |
| `EXPIRED` | Past expire_by timestamp |

## Metering

Record on each operation:
```rust
record_payment_link(&tenant_id, "created");
record_payment_link(&tenant_id, &status.to_string());
record_payment_link_amount(&tenant_id, &currency, amount);
```

## Acceptance Criteria

- [ ] CreatePaymentLink creates link in Razorpay
- [ ] CreatePaymentLink returns short_url
- [ ] CreatePaymentLink supports partial payment configuration
- [ ] CreatePaymentLink supports expiry configuration
- [ ] CreatePaymentLink validates amount is positive
- [ ] CreatePaymentLink validates min_partial_amount < amount
- [ ] GetPaymentLink returns link with current status
- [ ] GetPaymentLink returns NOT_FOUND for missing link
- [ ] CancelPaymentLink cancels CREATED link
- [ ] CancelPaymentLink cancels PARTIALLY_PAID link
- [ ] CancelPaymentLink returns FAILED_PRECONDITION for PAID link
- [ ] CancelPaymentLink returns FAILED_PRECONDITION for EXPIRED link
- [ ] ListPaymentLinks filters by status
- [ ] ListPaymentLinks pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create payment link with valid data returns link with short_url
- [ ] Create payment link with partial payment config succeeds
- [ ] Create payment link with expiry config succeeds
- [ ] Create payment link with zero amount returns INVALID_ARGUMENT
- [ ] Create payment link with min_partial >= amount returns INVALID_ARGUMENT
- [ ] Create payment link with past expire_by returns INVALID_ARGUMENT
- [ ] Get payment link returns complete link
- [ ] Get payment link returns NOT_FOUND for missing link
- [ ] Cancel created payment link succeeds
- [ ] Cancel partially paid payment link succeeds
- [ ] Cancel paid payment link returns FAILED_PRECONDITION
- [ ] Cancel expired payment link returns FAILED_PRECONDITION
- [ ] List payment links returns only tenant's links
- [ ] List payment links filters by status
- [ ] List payment links pagination works
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_payment_link_collection_full_flow` — Verifies [journey 006](../../../user-journeys/payment-service/006-payment-link-collection.md) steps 1-5: link created, short_url returned, customer pays, link marked as paid
- [ ] `journey_payment_link_partial_payment` — Verifies [journey 006](../../../user-journeys/payment-service/006-payment-link-collection.md) step 6: partial payment accepted, link status updated to PARTIALLY_PAID, subsequent payment completes
- [ ] `journey_payment_link_expiry` — Verifies [journey 006](../../../user-journeys/payment-service/006-payment-link-collection.md) step 9: expired link rejects further payments
