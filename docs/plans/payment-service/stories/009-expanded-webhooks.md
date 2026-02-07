# Story: Expanded Webhooks

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Expand the HandleRazorpayWebhook handler to process webhook events for transfers, settlements, subscriptions, linked accounts, payment links, and refunds. Route events to domain-specific handlers and update entity state accordingly.

## Tasks

- [ ] Define webhook event type enums for all domains
- [ ] Implement transfer webhook handlers (transfer.processed, transfer.reversed, transfer.failed)
- [ ] Implement settlement webhook handler (settlement.processed)
- [ ] Implement subscription webhook handlers (subscription.authenticated, .active, .pending, .halted, .paused, .resumed, .cancelled, .charged, .completed)
- [ ] Implement account webhook handlers (account.under_review, .needs_clarification, .activated, .suspended, .funds_hold, .funds_unhold)
- [ ] Implement payment link webhook handlers (payment_link.paid, .partially_paid, .cancelled, .expired)
- [ ] Implement refund webhook handlers (refund.created, .processed, .failed)
- [ ] Add event routing in HandleRazorpayWebhook to dispatch to domain handlers
- [ ] Add webhook signature verification for all new event types
- [ ] Add idempotency checks to prevent duplicate processing
- [ ] Add structured logging for all webhook events
- [ ] Add metering for webhook events by domain

## Webhook Events by Domain

### Payment Events (existing)
| Event | Handler Action |
|-------|---------------|
| `payment.authorized` | Update transaction status to PENDING |
| `payment.captured` | Update transaction status to COMPLETED |
| `payment.failed` | Update transaction status to FAILED |
| `order.paid` | Update transaction status to COMPLETED |

### Transfer Events
| Event | Handler Action |
|-------|---------------|
| `transfer.processed` | Update transfer status to PROCESSED |
| `transfer.reversed` | Update transfer status to REVERSED |
| `transfer.failed` | Update transfer status to FAILED |

### Settlement Events
| Event | Handler Action |
|-------|---------------|
| `settlement.processed` | Create/update settlement record with UTR, status PROCESSED |

### Subscription Events
| Event | Handler Action |
|-------|---------------|
| `subscription.authenticated` | Update subscription status to AUTHENTICATED |
| `subscription.active` | Update subscription status to ACTIVE |
| `subscription.pending` | Update subscription status to PENDING |
| `subscription.halted` | Update subscription status to HALTED |
| `subscription.paused` | Update subscription status to PAUSED |
| `subscription.resumed` | Update subscription status to ACTIVE |
| `subscription.cancelled` | Update subscription status to CANCELLED |
| `subscription.charged` | Record charge, update paid_count and remaining_count |
| `subscription.completed` | Update subscription status to COMPLETED |

### Account Events
| Event | Handler Action |
|-------|---------------|
| `account.under_review` | Update linked account status to UNDER_REVIEW |
| `account.needs_clarification` | Update linked account status to NEEDS_CLARIFICATION |
| `account.activated` | Update linked account status to ACTIVATED |
| `account.suspended` | Update linked account status to SUSPENDED |
| `account.funds_hold` | Record funds hold on linked account |
| `account.funds_unhold` | Record funds release on linked account |

### Payment Link Events
| Event | Handler Action |
|-------|---------------|
| `payment_link.paid` | Update payment link status to PAID, create transaction |
| `payment_link.partially_paid` | Update payment link status to PARTIALLY_PAID, create transaction |
| `payment_link.cancelled` | Update payment link status to CANCELLED |
| `payment_link.expired` | Update payment link status to EXPIRED |

### Refund Events
| Event | Handler Action |
|-------|---------------|
| `refund.created` | Create/update refund record |
| `refund.processed` | Update refund status to PROCESSED |
| `refund.failed` | Update refund status to FAILED |

## Idempotency

All webhook handlers must be idempotent:
- Use Razorpay entity IDs (razorpay_transfer_id, razorpay_settlement_id, etc.) for upsert operations
- Check current status before updating — skip if already in target state
- Log duplicate events at debug level

## Metering

Record on each webhook event:
```rust
record_webhook_event(&event_type, &domain);
```

## Acceptance Criteria

- [ ] HandleRazorpayWebhook routes events to correct domain handlers
- [ ] Webhook signature verification works for all event types
- [ ] Transfer webhooks update transfer status correctly
- [ ] Settlement webhooks create/update settlement records with UTR
- [ ] Subscription webhooks update subscription status and counts
- [ ] Account webhooks update linked account status
- [ ] Payment link webhooks update link status and create transactions
- [ ] Refund webhooks update refund status
- [ ] All handlers are idempotent (duplicate events handled gracefully)
- [ ] Unknown event types are logged and acknowledged (not rejected)
- [ ] All webhook processing is logged with structured fields

## Integration Tests

- [ ] Webhook with transfer.processed updates transfer to PROCESSED
- [ ] Webhook with transfer.reversed updates transfer to REVERSED
- [ ] Webhook with transfer.failed updates transfer to FAILED
- [ ] Webhook with settlement.processed creates settlement record
- [ ] Webhook with subscription.authenticated updates subscription
- [ ] Webhook with subscription.active updates subscription
- [ ] Webhook with subscription.charged updates counts
- [ ] Webhook with subscription.halted updates subscription
- [ ] Webhook with account.activated updates linked account
- [ ] Webhook with account.needs_clarification updates linked account
- [ ] Webhook with payment_link.paid updates link and creates transaction
- [ ] Webhook with payment_link.expired updates link status
- [ ] Webhook with refund.processed updates refund status
- [ ] Duplicate webhook event is handled idempotently
- [ ] Unknown webhook event is acknowledged (200 response)
- [ ] Invalid webhook signature returns UNAUTHENTICATED

## User Journey Integration Tests

- [ ] `journey_webhook_account_lifecycle` — Verifies [journey 001](../../../user-journeys/payment-service/001-tenant-onboarding.md) steps 4-6: account.under_review, account.needs_clarification, and account.activated webhooks update linked account status correctly
- [ ] `journey_webhook_subscription_lifecycle` — Verifies [journey 003](../../../user-journeys/payment-service/003-subscription-enrollment.md) steps 5, 7-9: subscription.authenticated, subscription.charged, subscription.pending, and subscription.halted webhooks drive subscription lifecycle
