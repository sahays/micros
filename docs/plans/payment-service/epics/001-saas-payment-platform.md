# Epic: SaaS Payment Platform

Status: planning
Created: 2026-02-07
Updated: 2026-02-07

## Overview

Evolve the payment-service into a SaaS payment platform using Razorpay Route. Enable platform tenants (linked accounts) to accept payments from their own customers, with configurable commission splitting, settlement management, recurring subscriptions, payment links, and refund processing.

## Core Principles

- Platform-first: All API calls via platform credentials with linked account references
- Commission-aware: Every payment supports configurable commission splitting
- Webhook-driven: Asynchronous lifecycle management via Razorpay webhooks
- Multi-tenant: Complete isolation via app_id/org_id scoping
- Service boundary: Payment-service owns Razorpay primitives; billing-service owns business logic

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- MongoDB
- Razorpay Route, Subscriptions, Payment Links APIs
- Prometheus metrics, OpenTelemetry tracing
- service-core for shared infrastructure

## Stories

### Phase 1: Foundation

- [ ] [001-linked-accounts](../stories/001-linked-accounts.md) - LinkedAccount CRUD, commission config, Razorpay Route account creation
- [ ] [002-customers](../stories/002-customers.md) - RazorpayCustomer CRUD, Razorpay customer API

### Phase 2: Payment Splitting

- [ ] [003-transfers](../stories/003-transfers.md) - Payment/order/direct transfers, reversal, listing
- [ ] [004-settlement-management](../stories/004-settlement-management.md) - Settlement hold/release, on-demand settlement, listing

### Phase 3: Recurring Payments

- [ ] [005-razorpay-plans](../stories/005-razorpay-plans.md) - Razorpay plan CRUD
- [ ] [006-razorpay-subscriptions](../stories/006-razorpay-subscriptions.md) - Subscription CRUD, pause, resume, cancel

### Phase 4: Payment Collection

- [ ] [007-payment-links](../stories/007-payment-links.md) - Payment link creation, cancellation, listing
- [ ] [008-refunds](../stories/008-refunds.md) - Refund initiation with transfer reversal support

### Phase 5: Webhooks & Events

- [ ] [009-expanded-webhooks](../stories/009-expanded-webhooks.md) - Handle transfer, settlement, subscription, account, payment link, and refund webhooks

### Phase 6: Observability

- [ ] [010-observability](../stories/010-observability.md) - Metrics, logging, tracing for all new domains

### Phase 7: API Consistency

- [ ] [011-standardize-amount-units](../stories/011-standardize-amount-units.md) - Standardize all amount fields to uint64 paise, eliminate float arithmetic

## gRPC Service: PaymentService

| Method | Story | Status |
|--------|-------|--------|
| CreateLinkedAccount | 001 | Planned |
| GetLinkedAccount | 001 | Planned |
| UpdateLinkedAccount | 001 | Planned |
| ListLinkedAccounts | 001 | Planned |
| UpdateCommissionConfig | 001 | Planned |
| CreateCustomer | 002 | Planned |
| GetCustomer | 002 | Planned |
| UpdateCustomer | 002 | Planned |
| ListCustomers | 002 | Planned |
| CreateTransferFromPayment | 003 | Planned |
| CreateTransferFromOrder | 003 | Planned |
| CreateDirectTransfer | 003 | Planned |
| ReverseTransfer | 003 | Planned |
| GetTransfer | 003 | Planned |
| ListTransfers | 003 | Planned |
| HoldTransferSettlement | 004 | Planned |
| ReleaseTransferSettlement | 004 | Planned |
| RequestOnDemandSettlement | 004 | Planned |
| GetSettlement | 004 | Planned |
| ListSettlements | 004 | Planned |
| CreateRazorpayPlan | 005 | Planned |
| GetRazorpayPlan | 005 | Planned |
| ListRazorpayPlans | 005 | Planned |
| CreateRazorpaySubscription | 006 | Planned |
| GetRazorpaySubscription | 006 | Planned |
| ListRazorpaySubscriptions | 006 | Planned |
| PauseRazorpaySubscription | 006 | Planned |
| ResumeRazorpaySubscription | 006 | Planned |
| CancelRazorpaySubscription | 006 | Planned |
| UpdateRazorpaySubscription | 006 | Planned |
| CreatePaymentLink | 007 | Planned |
| GetPaymentLink | 007 | Planned |
| CancelPaymentLink | 007 | Planned |
| ListPaymentLinks | 007 | Planned |
| InitiateRefund | 008 | Planned |
| GetRefund | 008 | Planned |
| ListRefunds | 008 | Planned |

## Capabilities

| Capability | Methods | Description |
|------------|---------|-------------|
| `payment.linked_account:create` | CreateLinkedAccount | Onboard linked accounts |
| `payment.linked_account:read` | GetLinkedAccount, ListLinkedAccounts | View linked accounts |
| `payment.linked_account:update` | UpdateLinkedAccount | Update linked account details |
| `payment.commission:manage` | UpdateCommissionConfig | Configure commission rates |
| `payment.customer:create` | CreateCustomer | Create customers |
| `payment.customer:read` | GetCustomer, ListCustomers | View customers |
| `payment.customer:update` | UpdateCustomer | Update customers |
| `payment.transfer:create` | CreateTransferFromPayment, CreateTransferFromOrder, CreateDirectTransfer | Create transfers |
| `payment.transfer:read` | GetTransfer, ListTransfers | View transfers |
| `payment.transfer:reverse` | ReverseTransfer | Reverse transfers |
| `payment.transfer:hold` | HoldTransferSettlement, ReleaseTransferSettlement | Manage settlement holds |
| `payment.settlement:create` | RequestOnDemandSettlement | Request settlements |
| `payment.settlement:read` | GetSettlement, ListSettlements | View settlements |
| `payment.plan:create` | CreateRazorpayPlan | Create Razorpay plans |
| `payment.plan:read` | GetRazorpayPlan, ListRazorpayPlans | View Razorpay plans |
| `payment.subscription:create` | CreateRazorpaySubscription | Create subscriptions |
| `payment.subscription:read` | GetRazorpaySubscription, ListRazorpaySubscriptions | View subscriptions |
| `payment.subscription:manage` | Pause, Resume, Cancel, UpdateRazorpaySubscription | Manage subscription lifecycle |
| `payment.payment_link:create` | CreatePaymentLink | Create payment links |
| `payment.payment_link:read` | GetPaymentLink, ListPaymentLinks | View payment links |
| `payment.payment_link:cancel` | CancelPaymentLink | Cancel payment links |
| `payment.refund:create` | InitiateRefund | Initiate refunds |
| `payment.refund:read` | GetRefund, ListRefunds | View refunds |

## Metering

Per-tenant payment platform metrics:
- `payment_linked_accounts_total{tenant_id, status}` - Linked accounts by status
- `payment_transfers_total{tenant_id, status}` - Transfers by status
- `payment_transfer_amount_total{tenant_id, currency}` - Transfer amounts
- `payment_commission_amount_total{tenant_id, currency}` - Commission collected
- `payment_settlements_total{tenant_id, type, status}` - Settlements by type and status
- `payment_subscriptions_total{tenant_id, status}` - Subscriptions by status
- `payment_subscription_charges_total{tenant_id, status}` - Subscription charges
- `payment_refunds_total{tenant_id, status, speed}` - Refunds by status and speed
- `payment_links_total{tenant_id, status}` - Payment links by status

## Dependencies

- **auth-service**: Organization and capability management
- **billing-service**: Orchestrates subscription lifecycle via payment-service
- **ledger-service**: Financial entries for transfers, settlements, refunds
- **notification-service**: Alerts for account status changes, subscription events

## Acceptance Criteria

- [ ] Linked account CRUD and Razorpay Route integration
- [ ] Commission configuration per linked account
- [ ] Customer CRUD and Razorpay customer integration
- [ ] Payment/order/direct transfer creation
- [ ] Transfer reversal
- [ ] Settlement hold, release, and on-demand
- [ ] Razorpay plan CRUD
- [ ] Razorpay subscription CRUD and lifecycle management
- [ ] Payment link creation and management
- [ ] Refund initiation with transfer reversal
- [ ] Webhook handling for all new event types
- [ ] Multi-tenant isolation for all new entities
- [ ] Prometheus metrics for all new domains
- [ ] OpenTelemetry tracing for all new operations
- [ ] All amount fields standardized to uint64 paise across the API
- [ ] All integration tests passing
- [ ] All user journey integration tests passing
