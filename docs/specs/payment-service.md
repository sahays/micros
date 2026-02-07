# Payment Service

**Multi-tenant SaaS payment platform with Razorpay integration, marketplace splits, subscriptions, and payment links.**

## Problem

Applications need payment processing capabilities: order creation, payment verification, refund handling, and usage tracking. Building custom payment integrations per app leads to:
- Inconsistent provider abstractions
- Security vulnerabilities in signature verification
- No centralized transaction tracking or audit trails
- No multi-tenant isolation

Additionally, SaaS platforms and marketplaces need their tenants (linked accounts) to accept payments from their own customers, with configurable commission splitting. Without a platform payment model:
- Each tenant must integrate payment providers independently
- Commission and settlement management is manual
- No unified view of platform-wide payment activity
- Subscription and recurring billing require separate infrastructure

## Solution

A reusable payment microservice providing:
- Multi-tenant transaction management
- Razorpay payment integration (orders, verification, webhooks)
- UPI QR code generation
- Per-tenant transaction isolation
- Webhook event handling with signature verification
- Provider abstraction for future payment gateways
- Razorpay Route integration for marketplace payment splitting
- Linked account (tenant) onboarding and KYC management
- Configurable commission splitting (percentage, fixed, or both)
- Transfer management with settlement hold/release
- Razorpay Subscriptions for recurring payment collection
- Payment links for one-time and partial payment collection
- Refund processing with automatic transfer reversal
- Customer management for recurring payment workflows

## Core Principles

- **Multi-tenant:** Complete isolation via app_id/org_id scoping
- **Provider abstraction:** Pluggable payment providers (Razorpay, UPI, future providers)
- **Secure by design:** HMAC signature verification for payments and webhooks
- **Webhook-driven:** Asynchronous event handling from payment providers
- **BFF trust model:** Trusts upstream services for authorization
- **Platform-first:** All Razorpay API calls made using platform credentials with linked account references
- **Commission-aware:** Every payment through a linked account supports configurable commission splitting

## Platform Payment Model

### Three-Party Model

The platform operates a three-party payment model:

1. **Platform** — The SaaS application (owns Razorpay account, makes all API calls)
2. **Tenant (Linked Account)** — A business onboarded onto the platform via Razorpay Route
3. **End Customer** — The tenant's customer who makes payments

Each org_id maps 1:1 to a Razorpay linked account. The platform retains a configurable commission on every payment processed through a linked account.

### Payment Flow

```
End Customer
    │
    │ Pays for product/service
    ▼
Razorpay (Platform Account)
    │
    ├─ Commission ──→ Platform Balance
    │
    └─ Transfer ────→ Linked Account (Tenant)
                         │
                         └─ Settlement ──→ Tenant Bank Account
```

### Commission Configuration

Commission is configured per linked account and supports three modes:
- **Percentage:** A percentage of the payment amount (e.g., 5%)
- **Fixed:** A fixed amount per transaction (e.g., 500 paise)
- **Both:** Percentage + fixed combined

Commission is deducted before the transfer amount is calculated.

## Data Model

### Transactions
- `id`: UUID
- `app_id`: Application/tenant identifier
- `org_id`: Organization identifier
- `user_id`: Optional user identifier
- `amount`: Payment amount (smallest currency unit)
- `currency`: ISO currency code (INR, USD)
- `status`: Transaction lifecycle state
- `provider_order_id`: External provider reference (Razorpay order ID)
- `linked_account_id`: Optional linked account reference for marketplace payments
- `subscription_id`: Optional subscription reference for recurring payments
- `customer_id`: Optional Razorpay customer reference
- `payment_link_id`: Optional payment link reference
- `created_at`: Timestamp
- `updated_at`: Timestamp

### Transaction Status
| Status | Description |
|--------|-------------|
| `CREATED` | Transaction created, awaiting payment |
| `PENDING` | Payment initiated, awaiting confirmation |
| `COMPLETED` | Payment successful |
| `FAILED` | Payment failed |
| `REFUNDED` | Payment fully refunded |
| `PARTIALLY_REFUNDED` | Payment partially refunded |

### Payment Methods
- `id`: UUID
- `app_id`: Tenant application ID
- `org_id`: Tenant organization ID
- `name`: Display name
- `provider`: Payment provider identifier
- `is_active`: Enabled status

### LinkedAccount
- `id`: UUID
- `org_id`: Organization identifier (1:1 mapping)
- `app_id`: Application identifier
- `razorpay_account_id`: Razorpay Route account ID
- `status`: Account lifecycle state
- `business_name`: Legal business name
- `business_type`: Type of business entity
- `legal_info`: PAN, GST, and other legal identifiers
- `bank_account`: Bank account details for settlements
- `settlement_schedule`: Settlement frequency and timing
- `commission_config`: Commission configuration for this account
- `created_at`: Timestamp
- `updated_at`: Timestamp

#### Linked Account Status
| Status | Description |
|--------|-------------|
| `CREATED` | Account created, not yet submitted to Razorpay |
| `UNDER_REVIEW` | Razorpay KYC review in progress |
| `NEEDS_CLARIFICATION` | Razorpay requires additional documents |
| `ACTIVATED` | Account verified, can accept payments |
| `SUSPENDED` | Account suspended by Razorpay or platform |

### CommissionConfig
- `type`: Commission type (percentage, fixed, both)
- `percentage_value`: Percentage value (e.g., 500 = 5.00%)
- `fixed_value`: Fixed amount in smallest currency unit
- `currency`: Currency for fixed commission

### Transfer
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_transfer_id`: Razorpay transfer ID
- `source_payment_id`: Source payment ID (for payment-level transfers)
- `source_order_id`: Source order ID (for order-level transfers)
- `linked_account_id`: Destination linked account
- `amount`: Transfer amount in smallest currency unit
- `currency`: ISO currency code
- `status`: Transfer lifecycle state
- `on_hold`: Whether settlement is on hold
- `on_hold_until`: Auto-release timestamp (if time-bound hold)
- `created_at`: Timestamp
- `updated_at`: Timestamp

#### Transfer Status
| Status | Description |
|--------|-------------|
| `CREATED` | Transfer created |
| `PENDING` | Transfer processing |
| `PROCESSED` | Transfer completed to linked account |
| `REVERSED` | Transfer reversed (refund scenario) |
| `FAILED` | Transfer failed |

### Settlement
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_settlement_id`: Razorpay settlement ID
- `linked_account_id`: Linked account reference
- `amount`: Settlement amount
- `fees`: Razorpay fees deducted
- `tax`: Tax on fees
- `type`: Settlement type (normal, instant, on_demand)
- `status`: Settlement lifecycle state
- `utr`: Bank UTR number for tracking
- `settled_at`: Settlement timestamp
- `created_at`: Timestamp

#### Settlement Status
| Status | Description |
|--------|-------------|
| `CREATED` | Settlement initiated |
| `PROCESSED` | Funds transferred to bank account |
| `FAILED` | Settlement failed |

### RazorpayCustomer
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_customer_id`: Razorpay customer ID
- `user_id`: Internal user reference
- `name`: Customer name
- `email`: Customer email
- `contact`: Customer phone number
- `created_at`: Timestamp
- `updated_at`: Timestamp

### RazorpayPlan
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_plan_id`: Razorpay plan ID
- `name`: Plan name
- `description`: Plan description
- `period`: Billing period (daily, weekly, monthly, yearly)
- `interval`: Number of periods between charges
- `amount`: Charge amount in smallest currency unit
- `currency`: ISO currency code
- `created_at`: Timestamp

### RazorpaySubscription
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_subscription_id`: Razorpay subscription ID
- `plan_id`: Reference to RazorpayPlan
- `customer_id`: Reference to RazorpayCustomer
- `status`: Subscription lifecycle state
- `current_start`: Current billing period start
- `current_end`: Current billing period end
- `total_count`: Total number of charges planned
- `paid_count`: Number of charges completed
- `remaining_count`: Number of charges remaining
- `short_url`: Razorpay-hosted subscription authorization URL
- `created_at`: Timestamp
- `updated_at`: Timestamp

#### Razorpay Subscription Status
| Status | Description |
|--------|-------------|
| `CREATED` | Subscription created, awaiting authorization |
| `AUTHENTICATED` | Customer authorized recurring charges |
| `ACTIVE` | First charge completed, subscription active |
| `PENDING` | Charge attempt failed, retrying |
| `HALTED` | All retry attempts exhausted |
| `PAUSED` | Subscription paused by tenant |
| `CANCELLED` | Subscription cancelled |
| `COMPLETED` | All planned charges completed |

### PaymentLink
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_payment_link_id`: Razorpay payment link ID
- `amount`: Payment amount in smallest currency unit
- `currency`: ISO currency code
- `description`: Payment description
- `short_url`: Shortened payment URL
- `status`: Payment link lifecycle state
- `accept_partial`: Whether partial payments are accepted
- `min_partial_amount`: Minimum partial payment amount
- `expire_by`: Expiry timestamp
- `created_at`: Timestamp
- `updated_at`: Timestamp

#### Payment Link Status
| Status | Description |
|--------|-------------|
| `CREATED` | Payment link created |
| `PARTIALLY_PAID` | Partial payment received |
| `PAID` | Full payment received |
| `CANCELLED` | Payment link cancelled |
| `EXPIRED` | Payment link expired |

### Refund
- `id`: UUID
- `app_id`: Application identifier
- `org_id`: Organization identifier
- `razorpay_refund_id`: Razorpay refund ID
- `payment_id`: Reference to original payment/transaction
- `transaction_id`: Reference to transaction record
- `amount`: Refund amount in smallest currency unit
- `currency`: ISO currency code
- `speed`: Refund speed (normal, optimum)
- `status`: Refund lifecycle state
- `reverse_all_transfers`: Whether to reverse linked account transfers
- `created_at`: Timestamp
- `updated_at`: Timestamp

#### Refund Status
| Status | Description |
|--------|-------------|
| `CREATED` | Refund initiated |
| `PROCESSED` | Refund completed |
| `FAILED` | Refund failed |

## gRPC Service: PaymentService

### Core Payment Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreateTransaction` | Unary | Create a new transaction record |
| `GetTransaction` | Unary | Retrieve transaction by ID |
| `UpdateTransactionStatus` | Unary | Update transaction status |
| `ListTransactions` | Unary | List transactions with pagination |
| `CreateRazorpayOrder` | Unary | Create Razorpay payment order |
| `VerifyRazorpayPayment` | Unary | Verify payment signature |
| `GenerateUpiQr` | Unary | Generate UPI payment QR code |
| `HandleRazorpayWebhook` | Unary | Process Razorpay webhook events |

### Linked Account Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreateLinkedAccount` | Unary | Create linked account via Razorpay Route |
| `GetLinkedAccount` | Unary | Retrieve linked account details |
| `UpdateLinkedAccount` | Unary | Update linked account information |
| `ListLinkedAccounts` | Unary | List linked accounts with filters |
| `UpdateCommissionConfig` | Unary | Update commission configuration for linked account |

### Transfer Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreateTransferFromPayment` | Unary | Create transfer from a captured payment |
| `CreateTransferFromOrder` | Unary | Create transfer at order level (auto-split on capture) |
| `CreateDirectTransfer` | Unary | Transfer from platform balance to linked account |
| `ReverseTransfer` | Unary | Reverse a processed transfer |
| `GetTransfer` | Unary | Retrieve transfer details |
| `ListTransfers` | Unary | List transfers with filters |
| `HoldTransferSettlement` | Unary | Place settlement hold on a transfer |
| `ReleaseTransferSettlement` | Unary | Release settlement hold on a transfer |

### Settlement Methods
| Method | Type | Description |
|--------|------|-------------|
| `RequestOnDemandSettlement` | Unary | Request instant or on-demand settlement |
| `GetSettlement` | Unary | Retrieve settlement details |
| `ListSettlements` | Unary | List settlements with filters |

### Subscription Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreateRazorpayPlan` | Unary | Create a plan in Razorpay |
| `GetRazorpayPlan` | Unary | Retrieve Razorpay plan details |
| `ListRazorpayPlans` | Unary | List Razorpay plans |
| `CreateRazorpaySubscription` | Unary | Create subscription in Razorpay |
| `GetRazorpaySubscription` | Unary | Retrieve subscription details |
| `ListRazorpaySubscriptions` | Unary | List subscriptions with filters |
| `PauseRazorpaySubscription` | Unary | Pause an active subscription |
| `ResumeRazorpaySubscription` | Unary | Resume a paused subscription |
| `CancelRazorpaySubscription` | Unary | Cancel a subscription |
| `UpdateRazorpaySubscription` | Unary | Update subscription parameters |

### Customer Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreateCustomer` | Unary | Create customer in Razorpay |
| `GetCustomer` | Unary | Retrieve customer details |
| `UpdateCustomer` | Unary | Update customer information |
| `ListCustomers` | Unary | List customers with filters |

### Payment Link Methods
| Method | Type | Description |
|--------|------|-------------|
| `CreatePaymentLink` | Unary | Create Razorpay payment link |
| `GetPaymentLink` | Unary | Retrieve payment link details |
| `CancelPaymentLink` | Unary | Cancel an active payment link |
| `ListPaymentLinks` | Unary | List payment links with filters |

### Refund Methods
| Method | Type | Description |
|--------|------|-------------|
| `InitiateRefund` | Unary | Initiate a full or partial refund |
| `GetRefund` | Unary | Retrieve refund details |
| `ListRefunds` | Unary | List refunds with filters |

## Razorpay Integration

### Order Creation Flow
```
Client → BFF → CreateRazorpayOrder
                    │
                    ├─1→ Create Razorpay order (API)
                    ├─2→ Store transaction with provider_order_id
                    │
                    └──→ Return order_id + key_id for checkout
```

### Payment Verification Flow
```
Client (Razorpay callback) → BFF → VerifyRazorpayPayment
                                        │
                                        ├─1→ Fetch transaction by ID
                                        ├─2→ Verify order_id matches
                                        ├─3→ Verify HMAC signature
                                        ├─4→ Update status (Completed/Failed)
                                        │
                                        └──→ Return verification result
```

### Webhook Flow
```
Razorpay → HandleRazorpayWebhook
               │
               ├─1→ Verify webhook signature
               ├─2→ Parse event type and route to handler
               ├─3→ Update relevant entity (transaction, transfer, subscription, etc.)
               │
               └──→ Acknowledge webhook
```

### Linked Account Onboarding Flow
```
Platform Admin → CreateLinkedAccount
                      │
                      ├─1→ Validate business details
                      ├─2→ Create Razorpay Route linked account (API)
                      ├─3→ Store linked account with razorpay_account_id
                      ├─4→ Status: CREATED → UNDER_REVIEW (via webhook)
                      │
                      └──→ Return linked account details

Razorpay Webhooks:
  account.under_review  → Update status to UNDER_REVIEW
  account.needs_clarification → Update status to NEEDS_CLARIFICATION
  account.activated     → Update status to ACTIVATED
  account.suspended     → Update status to SUSPENDED
```

### Payment with Automatic Split Flow
```
Client → BFF → CreateRazorpayOrder (with transfer config)
                    │
                    ├─1→ Create order with transfers[] (linked_account, amount)
                    ├─2→ Commission calculated: payment_amount - transfer_amount
                    ├─3→ Customer pays → payment.captured webhook
                    ├─4→ Transfers auto-created by Razorpay
                    ├─5→ transfer.processed webhook → update transfer records
                    │
                    └──→ Platform retains commission, tenant receives transfer
```

### Payment with Post-Capture Split Flow
```
Payment captured → CreateTransferFromPayment
                        │
                        ├─1→ Verify payment is captured
                        ├─2→ Calculate transfer amount (payment - commission)
                        ├─3→ Create transfer via Razorpay API
                        ├─4→ Store transfer record
                        │
                        └──→ Return transfer details
```

### Direct Transfer Flow
```
Platform → CreateDirectTransfer
               │
               ├─1→ Verify platform balance sufficient
               ├─2→ Create transfer to linked account (Razorpay API)
               ├─3→ Store transfer record
               │
               └──→ Return transfer details
```

### Settlement Flow
```
Transfer processed → Settlement scheduled per linked account config
                          │
                          ├─ Normal: T+2 default settlement
                          ├─ Instant: RequestOnDemandSettlement
                          │
                          └──→ settlement.processed webhook
                                   │
                                   ├─ Record settlement with UTR
                                   └─ Update linked account balance
```

### Subscription Authorization Flow
```
Tenant → CreateRazorpayPlan → CreateRazorpaySubscription
                                    │
                                    ├─1→ Create subscription in Razorpay
                                    ├─2→ Return short_url for customer authorization
                                    │
Customer → Opens short_url → Authorizes recurring charges
                                    │
                                    ├─ subscription.authenticated webhook
                                    ├─ First charge executes
                                    ├─ subscription.active webhook
                                    │
Recurring charges:
  ├─ subscription.charged → Record payment, update cycle
  ├─ Charge fails → Retry T+1, T+2, T+3
  ├─ subscription.pending → Notify tenant
  └─ subscription.halted → All retries exhausted
```

### Refund with Transfer Reversal Flow
```
Tenant → InitiateRefund
              │
              ├─1→ Validate refund amount <= captured - prior refunds
              ├─2→ If reverse_all_transfers: reverse linked account transfers
              ├─3→ Submit refund to Razorpay
              ├─4→ refund.created webhook → store refund record
              ├─5→ refund.processed webhook → update status
              ├─6→ transfer.reversed webhook → update transfer records
              │
              └──→ Transaction status: REFUNDED or PARTIALLY_REFUNDED
```

## UPI Integration

Generate UPI payment intent URLs and QR codes:
- Format: `upi://pay?pa={vpa}&pn={merchant_name}&am={amount}&cu=INR&tn={description}`
- QR code returned as base64-encoded PNG

## Authentication Model

### Request Metadata
All requests require tenant context headers:
- `x-app-id`: Required application/tenant ID
- `x-org-id`: Required organization ID
- `x-user-id`: Optional user ID

### Trust Model
Payment-service uses a **BFF trust model**:
- Trusts upstream services to validate authorization
- Does NOT validate JWT tokens directly
- Multi-tenant isolation via database queries with app_id/org_id scoping

## Capabilities

Capabilities control access to payment-service operations.

**Format:** `{domain}.{resource}:{action}`

### Core Payment Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.transaction:create` | CreateTransaction | Create transactions |
| `payment.transaction:read` | GetTransaction, ListTransactions | View transactions |
| `payment.transaction:update` | UpdateTransactionStatus | Update transaction status |
| `payment.razorpay:create` | CreateRazorpayOrder | Create Razorpay orders |
| `payment.razorpay:verify` | VerifyRazorpayPayment | Verify payment signatures |
| `payment.upi:generate` | GenerateUpiQr | Generate UPI QR codes |
| `payment.webhook:handle` | HandleRazorpayWebhook | Process webhooks |

### Linked Account Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.linked_account:create` | CreateLinkedAccount | Onboard linked accounts |
| `payment.linked_account:read` | GetLinkedAccount, ListLinkedAccounts | View linked accounts |
| `payment.linked_account:update` | UpdateLinkedAccount | Update linked account details |
| `payment.commission:manage` | UpdateCommissionConfig | Configure commission rates |

### Transfer Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.transfer:create` | CreateTransferFromPayment, CreateTransferFromOrder, CreateDirectTransfer | Create transfers |
| `payment.transfer:read` | GetTransfer, ListTransfers | View transfers |
| `payment.transfer:reverse` | ReverseTransfer | Reverse transfers |
| `payment.transfer:hold` | HoldTransferSettlement, ReleaseTransferSettlement | Manage settlement holds |

### Settlement Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.settlement:create` | RequestOnDemandSettlement | Request settlements |
| `payment.settlement:read` | GetSettlement, ListSettlements | View settlements |

### Subscription Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.plan:create` | CreateRazorpayPlan | Create Razorpay plans |
| `payment.plan:read` | GetRazorpayPlan, ListRazorpayPlans | View Razorpay plans |
| `payment.subscription:create` | CreateRazorpaySubscription | Create subscriptions |
| `payment.subscription:read` | GetRazorpaySubscription, ListRazorpaySubscriptions | View subscriptions |
| `payment.subscription:manage` | PauseRazorpaySubscription, ResumeRazorpaySubscription, CancelRazorpaySubscription, UpdateRazorpaySubscription | Manage subscription lifecycle |

### Customer Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.customer:create` | CreateCustomer | Create customers |
| `payment.customer:read` | GetCustomer, ListCustomers | View customers |
| `payment.customer:update` | UpdateCustomer | Update customers |

### Payment Link Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.payment_link:create` | CreatePaymentLink | Create payment links |
| `payment.payment_link:read` | GetPaymentLink, ListPaymentLinks | View payment links |
| `payment.payment_link:cancel` | CancelPaymentLink | Cancel payment links |

### Refund Capabilities
| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.refund:create` | InitiateRefund | Initiate refunds |
| `payment.refund:read` | GetRefund, ListRefunds | View refunds |

### Capability Enforcement Modes

**1. BFF Trust Model (Default)**
- When `AUTH_SERVICE_ENDPOINT` is not configured
- Trusts upstream services for authorization
- Capability enforcement handled by secure-frontend

**2. Direct Capability Enforcement (Optional)**
- When `AUTH_SERVICE_ENDPOINT` is configured
- Validates JWT tokens via auth-service
- Checks capabilities for each gRPC method

## Webhook Handling

### Payment Events
| Event | Action |
|-------|--------|
| `payment.authorized` | Update transaction status to PENDING |
| `payment.captured` | Update transaction status to COMPLETED |
| `payment.failed` | Update transaction status to FAILED |
| `order.paid` | Update transaction status to COMPLETED |

### Transfer Events
| Event | Action |
|-------|--------|
| `transfer.processed` | Update transfer status to PROCESSED |
| `transfer.reversed` | Update transfer status to REVERSED |
| `transfer.failed` | Update transfer status to FAILED |

### Settlement Events
| Event | Action |
|-------|--------|
| `settlement.processed` | Record settlement with UTR, update status to PROCESSED |

### Subscription Events
| Event | Action |
|-------|--------|
| `subscription.authenticated` | Update subscription status to AUTHENTICATED |
| `subscription.active` | Update subscription status to ACTIVE |
| `subscription.pending` | Update subscription status to PENDING (retry in progress) |
| `subscription.halted` | Update subscription status to HALTED |
| `subscription.paused` | Update subscription status to PAUSED |
| `subscription.resumed` | Update subscription status to ACTIVE |
| `subscription.cancelled` | Update subscription status to CANCELLED |
| `subscription.charged` | Record charge, update paid_count/remaining_count |
| `subscription.completed` | Update subscription status to COMPLETED |

### Account Events
| Event | Action |
|-------|--------|
| `account.under_review` | Update linked account status to UNDER_REVIEW |
| `account.needs_clarification` | Update linked account status to NEEDS_CLARIFICATION |
| `account.activated` | Update linked account status to ACTIVATED |
| `account.suspended` | Update linked account status to SUSPENDED |
| `account.funds_hold` | Record funds hold on linked account |
| `account.funds_unhold` | Record funds release on linked account |

### Payment Link Events
| Event | Action |
|-------|--------|
| `payment_link.paid` | Update payment link status to PAID |
| `payment_link.partially_paid` | Update payment link status to PARTIALLY_PAID |
| `payment_link.cancelled` | Update payment link status to CANCELLED |
| `payment_link.expired` | Update payment link status to EXPIRED |

### Refund Events
| Event | Action |
|-------|--------|
| `refund.created` | Store refund record |
| `refund.processed` | Update refund status to PROCESSED |
| `refund.failed` | Update refund status to FAILED |

## Service Boundary: Payment vs. Billing

The payment-service and billing-service have distinct responsibilities:

**Payment-service** owns Razorpay primitives:
- Creates and manages Razorpay plans, subscriptions, and customers
- Executes charges, processes webhooks, manages lifecycle
- Handles transfers, settlements, refunds, and payment links
- Stores Razorpay entity state and maps to internal IDs

**Billing-service** owns business logic:
- Defines billing plans with proration, usage components, and pricing tiers
- Manages usage aggregation and billing cycles
- Orchestrates billing runs and invoice generation
- Calls payment-service to create/manage Razorpay subscriptions

The billing-service orchestrates the payment-service for recurring payment execution.

## Business Rules

1. One linked account per org_id — each org maps to exactly one Razorpay merchant
2. Transfers must not exceed the captured payment amount
3. Commission is deducted before transfer amount is calculated
4. Settlement holds can be time-bound (auto-release after on_hold_until) or indefinite (manual release)
5. Subscription charge retry schedule: T+1, T+2, T+3, then halt
6. Refund amount cannot exceed captured amount minus prior refunds
7. All Razorpay API calls use platform credentials with account_id parameter
8. All amounts are in the smallest currency unit (paise for INR, cents for USD)
9. Linked account must be in ACTIVATED status to receive transfers
10. Payment links can only be cancelled if status is CREATED or PARTIALLY_PAID
11. Partial refunds create PARTIALLY_REFUNDED status; full refunds create REFUNDED status

## Integration Pattern

```
Client App
    │
    │ Authorization: Bearer <access_token>
    ▼
BFF (secure-frontend)
    │
    │ 1. Validate JWT token
    │ 2. Check payment:* capabilities
    │ 3. Add tenant headers (x-app-id, x-org-id, x-user-id)
    ▼
Payment Service
    │
    │ Trust caller, process request with tenant scoping
    ▼
Razorpay API + MongoDB
```

## Use Cases

- **Online payments:** Create orders, verify payments, handle callbacks
- **QR payments:** Generate UPI payment links and QR codes
- **Transaction tracking:** List and filter transactions by status
- **Refund handling:** Process full and partial refunds with transfer reversal
- **Audit trail:** Complete transaction history per tenant
- **Marketplace payments:** Split payments between platform and linked accounts with commission
- **SaaS subscriptions:** Create and manage recurring payment subscriptions via Razorpay
- **Vendor payouts:** Transfer funds from platform to linked accounts (direct transfers)
- **Split refunds:** Refund customers with automatic reversal of linked account transfers
- **Payment links:** Generate shareable payment links for one-time or partial collection

## Key Features

- **Multi-tenant isolation:** All queries scoped by app_id + org_id
- **Signature verification:** HMAC-SHA256 for payments and webhooks
- **Webhook idempotency:** Update by provider IDs prevents duplicates
- **Provider abstraction:** Easy to add new payment providers
- **Health endpoints:** HTTP /health, /ready, /metrics
- **gRPC reflection:** Debugging via grpcurl
- **Commission splitting:** Configurable per linked account
- **Settlement management:** Hold, release, and on-demand settlement support

## Edge Cases

- **Invalid transaction ID:** Returns InvalidArgument
- **Transaction not found:** Returns NotFound
- **Razorpay not configured:** Returns FailedPrecondition
- **Invalid signature:** Returns Unauthenticated (webhooks), verification failure (payments)
- **Order ID mismatch:** Returns InvalidArgument
- **Missing tenant headers:** Returns Unauthenticated
- **Database error:** Returns Internal
- **Linked account not activated:** Returns FailedPrecondition when attempting transfers to non-activated accounts
- **Insufficient platform balance:** Returns FailedPrecondition for direct transfers exceeding available balance
- **Halted subscription:** Returns FailedPrecondition when attempting to charge a halted subscription; requires manual intervention
- **Expired payment link:** Returns FailedPrecondition; no further payments accepted after expiry
- **Suspended linked account:** Returns FailedPrecondition; no transfers or settlements processed
- **Transfer exceeds captured amount:** Returns InvalidArgument when transfer amount exceeds remaining captured balance
- **Refund exceeds available amount:** Returns InvalidArgument when refund amount exceeds captured minus prior refunds

## Non-Goals

- Direct user authentication (use auth-service)
- Billing business logic, proration, usage aggregation (use billing-service)
- Ledger entries (use ledger-service)
- Invoice generation (use invoicing-service)
- Direct merchant dashboard (tenants use platform UI)

## Observability

### HTTP Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Liveness probe |
| `GET /ready` | Readiness probe |
| `GET /metrics` | Prometheus metrics |

### Logging
- Structured JSON to stdout (PLG-compatible)
- Transaction lifecycle logging with IDs
- Webhook event logging by domain (payment, transfer, subscription, account, refund)
- Provider API call logging
- Linked account status change logging
- Transfer and settlement lifecycle logging

### Metrics

**gRPC Metrics:**
- `grpc_requests_total` - Requests by method, status
- `grpc_request_duration_seconds` - Duration histogram by method
- `grpc_requests_in_flight` - Current request count by method
- `grpc_metering_total` - Requests by tenant_id (via interceptor)

**Payment Metrics (Per-Tenant Billing):**
- `payment_transactions_total{tenant_id, status}` - Transactions by tenant and status
- `payment_amount_total{tenant_id, currency}` - Total payment amounts by tenant
- `payment_razorpay_requests_total{tenant_id, operation}` - Razorpay API calls by tenant
- `payment_webhook_events_total{event_type, domain}` - Webhook events by type and domain

**Transfer Metrics:**
- `payment_transfers_total{tenant_id, status}` - Transfers by tenant and status
- `payment_transfer_amount_total{tenant_id, currency}` - Total transfer amounts
- `payment_transfer_reversals_total{tenant_id}` - Transfer reversals count

**Settlement Metrics:**
- `payment_settlements_total{tenant_id, type, status}` - Settlements by type and status
- `payment_settlement_amount_total{tenant_id, currency}` - Total settlement amounts

**Commission Metrics:**
- `payment_commission_amount_total{tenant_id, currency}` - Total commission collected
- `payment_commission_config_updates_total{tenant_id}` - Commission config changes

**Subscription Metrics:**
- `payment_subscriptions_total{tenant_id, status}` - Subscriptions by status
- `payment_subscription_charges_total{tenant_id, status}` - Subscription charges by outcome
- `payment_subscription_retries_total{tenant_id}` - Charge retry attempts

**Refund Metrics:**
- `payment_refunds_total{tenant_id, status, speed}` - Refunds by status and speed
- `payment_refund_amount_total{tenant_id, currency}` - Total refund amounts

**Linked Account Metrics:**
- `payment_linked_accounts_total{tenant_id, status}` - Linked accounts by status
- `payment_linked_account_activations_total{tenant_id}` - Activation count

**Payment Link Metrics:**
- `payment_links_total{tenant_id, status}` - Payment links by status
- `payment_link_amount_total{tenant_id, currency}` - Total payment link amounts

**Database Metrics:**
- `db_operation_duration_seconds` - Operation latency by operation, collection
- `db_errors_total` - Database errors by operation, collection

### Billing and Metering

Per-tenant usage is tracked through:
1. **Prometheus metrics:** All payment metrics include `tenant_id` label
2. **MongoDB collections:** Complete history with app_id, org_id
3. **gRPC metering interceptor:** `grpc_metering_total` counter from service-core

### Tracing
- OpenTelemetry spans for all operations
- Trace ID propagation to Razorpay calls
- Tenant/transaction/transfer/subscription context in spans
- Exports to Tempo via OTLP/gRPC

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `PAYMENT_SERVICE_HOST` | HTTP listen address | `0.0.0.0` |
| `PAYMENT_SERVICE_PORT` | HTTP port | `3003` |
| `PAYMENT_SERVICE_GRPC_PORT` | gRPC port | `3004` |
| `PAYMENT_DATABASE_URL` | MongoDB connection string | (required) |
| `PAYMENT_DATABASE_NAME` | Database name | `payment_db` |
| `PAYMENT_REDIS_URL` | Redis connection URI | `redis://localhost:6379` |
| `RAZORPAY_KEY_ID` | Razorpay API key ID | (optional) |
| `RAZORPAY_KEY_SECRET` | Razorpay API secret | (optional) |
| `RAZORPAY_WEBHOOK_SECRET` | Webhook verification secret | (optional) |
| `RAZORPAY_API_BASE_URL` | Razorpay API endpoint | `https://api.razorpay.com/v1` |
| `PAYMENT_UPI_VPA` | Default UPI Virtual Payment Address | `merchant@upi` |
| `PAYMENT_UPI_MERCHANT_NAME` | Default merchant name | `Micros Merchant` |
| `AUTH_SERVICE_ENDPOINT` | Auth-service endpoint (enables capability enforcement) | (unset) |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://tempo:4317` |
| `RAZORPAY_ROUTE_ENABLED` | Enable Razorpay Route features (linked accounts, transfers, settlements) | `false` |
| `RAZORPAY_SUBSCRIPTIONS_ENABLED` | Enable Razorpay Subscriptions features | `false` |
| `DEFAULT_COMMISSION_PERCENTAGE` | Default commission percentage for new linked accounts (basis points) | `500` |
| `DEFAULT_SETTLEMENT_SCHEDULE` | Default settlement schedule for linked accounts | `T+2` |

## Database Indexes

Compound indexes for tenant-scoped queries:

### Transactions
- `(app_id, org_id, _id)` - Transaction lookup
- `(app_id, org_id, user_id)` - User-scoped queries
- `(app_id, org_id, status)` - Status filtering
- `(provider_order_id)` - Webhook lookups
- `(app_id, org_id, linked_account_id)` - Linked account transaction queries
- `(app_id, org_id, subscription_id)` - Subscription transaction queries
- `(app_id, org_id, payment_link_id)` - Payment link transaction queries

### Linked Accounts
- `(app_id, org_id)` - Unique per org (1:1 mapping)
- `(razorpay_account_id)` - Razorpay webhook lookups
- `(app_id, org_id, status)` - Status filtering

### Transfers
- `(app_id, org_id, _id)` - Transfer lookup
- `(razorpay_transfer_id)` - Webhook lookups
- `(app_id, org_id, linked_account_id)` - Per linked account
- `(source_payment_id)` - Payment-level transfer queries
- `(source_order_id)` - Order-level transfer queries

### Settlements
- `(app_id, org_id, _id)` - Settlement lookup
- `(razorpay_settlement_id)` - Webhook lookups
- `(app_id, org_id, linked_account_id)` - Per linked account

### Customers
- `(app_id, org_id, _id)` - Customer lookup
- `(razorpay_customer_id)` - Razorpay lookups
- `(app_id, org_id, user_id)` - User mapping

### Plans
- `(app_id, org_id, _id)` - Plan lookup
- `(razorpay_plan_id)` - Razorpay lookups

### Subscriptions
- `(app_id, org_id, _id)` - Subscription lookup
- `(razorpay_subscription_id)` - Webhook lookups
- `(app_id, org_id, customer_id)` - Per customer
- `(app_id, org_id, status)` - Status filtering

### Payment Links
- `(app_id, org_id, _id)` - Payment link lookup
- `(razorpay_payment_link_id)` - Webhook lookups
- `(app_id, org_id, status)` - Status filtering

### Refunds
- `(app_id, org_id, _id)` - Refund lookup
- `(razorpay_refund_id)` - Webhook lookups
- `(app_id, org_id, payment_id)` - Per payment

## Payment Providers

| Provider | Status | Capabilities |
|----------|--------|--------------|
| **Razorpay** | Implemented | Orders, payments, webhooks, refunds, Route (linked accounts, transfers, settlements), subscriptions, payment links |
| **UPI** | Implemented | QR codes, payment links |

## Implementation Files

| File | Description |
|------|-------------|
| `src/main.rs` | Entry point, initializes metrics and tracing |
| `src/startup.rs` | Application lifecycle, HTTP + gRPC server setup |
| `src/config/mod.rs` | Configuration structs and environment loading |
| `src/grpc/payment_service.rs` | gRPC method implementations |
| `src/grpc/capability_check.rs` | Capability enforcement module |
| `src/services/razorpay.rs` | Razorpay API client |
| `src/services/upi.rs` | UPI QR code generation |
| `src/services/repository.rs` | MongoDB repository |
| `src/services/metrics.rs` | Per-tenant metrics (Prometheus) |
| `src/models/mod.rs` | Data models and protobuf conversions |
| `tests/payment_test.rs` | Integration tests |
| `tests/common/mod.rs` | Test setup and helpers |

## References

- Proto Definition: `proto/micros/payment/v1/payment.proto`
- Razorpay API: https://razorpay.com/docs/api/
- Razorpay Route: https://razorpay.com/docs/api/route/
- Razorpay Subscriptions: https://razorpay.com/docs/api/subscriptions/
- Razorpay Payment Links: https://razorpay.com/docs/api/payment-links/
- UPI Specification: https://www.npci.org.in/what-we-do/upi/product-overview
