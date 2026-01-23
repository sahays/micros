# Payment Service

**Multi-tenant payment processing with Razorpay integration and UPI support.**

## Problem

Applications need payment processing capabilities: order creation, payment verification, refund handling, and usage tracking. Building custom payment integrations per app leads to:
- Inconsistent provider abstractions
- Security vulnerabilities in signature verification
- No centralized transaction tracking or audit trails
- No multi-tenant isolation

## Solution

A reusable payment microservice providing:
- Multi-tenant transaction management
- Razorpay payment integration (orders, verification, webhooks)
- UPI QR code generation
- Per-tenant transaction isolation
- Webhook event handling with signature verification
- Provider abstraction for future payment gateways

## Core Principles

- **Multi-tenant:** Complete isolation via app_id/org_id scoping
- **Provider abstraction:** Pluggable payment providers (Razorpay, UPI, future providers)
- **Secure by design:** HMAC signature verification for payments and webhooks
- **Webhook-driven:** Asynchronous event handling from payment providers
- **BFF trust model:** Trusts upstream services for authorization

## Data Model

### Transactions
- `id`: UUID
- `app_id`: Application/tenant identifier
- `org_id`: Organization identifier
- `user_id`: Optional user identifier
- `amount`: Payment amount (base currency units)
- `currency`: ISO currency code (INR, USD)
- `status`: Transaction lifecycle state
- `provider_order_id`: External provider reference (Razorpay order ID)
- `created_at`: Timestamp
- `updated_at`: Timestamp

### Transaction Status
| Status | Description |
|--------|-------------|
| `CREATED` | Transaction created, awaiting payment |
| `PENDING` | Payment initiated, awaiting confirmation |
| `COMPLETED` | Payment successful |
| `FAILED` | Payment failed |
| `REFUNDED` | Payment refunded |

### Payment Methods
- `id`: UUID
- `app_id`: Tenant application ID
- `org_id`: Tenant organization ID
- `name`: Display name
- `provider`: Payment provider identifier
- `is_active`: Enabled status

## gRPC Service: PaymentService

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
               ├─2→ Parse event (payment.captured, payment.failed, etc.)
               ├─3→ Update transaction by provider_order_id
               │
               └──→ Acknowledge webhook
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

| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `payment.transaction:create` | CreateTransaction | Create transactions |
| `payment.transaction:read` | GetTransaction, ListTransactions | View transactions |
| `payment.transaction:update` | UpdateTransactionStatus | Update transaction status |
| `payment.razorpay:create` | CreateRazorpayOrder | Create Razorpay orders |
| `payment.razorpay:verify` | VerifyRazorpayPayment | Verify payment signatures |
| `payment.upi:generate` | GenerateUpiQr | Generate UPI QR codes |
| `payment.webhook:handle` | HandleRazorpayWebhook | Process webhooks |

### Capability Enforcement Modes

**1. BFF Trust Model (Default)**
- When `AUTH_SERVICE_ENDPOINT` is not configured
- Trusts upstream services for authorization
- Capability enforcement handled by secure-frontend

**2. Direct Capability Enforcement (Optional)**
- When `AUTH_SERVICE_ENDPOINT` is configured
- Validates JWT tokens via auth-service
- Checks capabilities for each gRPC method

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
- **Refund handling:** Process refund webhooks from providers
- **Audit trail:** Complete transaction history per tenant

## Key Features

- **Multi-tenant isolation:** All queries scoped by app_id + org_id
- **Signature verification:** HMAC-SHA256 for payments and webhooks
- **Webhook idempotency:** Update by provider_order_id prevents duplicates
- **Provider abstraction:** Easy to add new payment providers
- **Health endpoints:** HTTP /health, /ready, /metrics
- **gRPC reflection:** Debugging via grpcurl

## Edge Cases

- **Invalid transaction ID:** Returns InvalidArgument
- **Transaction not found:** Returns NotFound
- **Razorpay not configured:** Returns FailedPrecondition
- **Invalid signature:** Returns Unauthenticated (webhooks), verification failure (payments)
- **Order ID mismatch:** Returns InvalidArgument
- **Missing tenant headers:** Returns Unauthenticated
- **Database error:** Returns Internal

## Non-Goals

- Direct user authentication (use auth-service)
- Billing/invoicing (use invoicing-service)
- Ledger entries (use ledger-service)
- Subscription management (future service)

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
- Webhook event logging
- Provider API call logging

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
- `payment_webhook_events_total{event_type}` - Webhook events by type

**Database Metrics:**
- `db_operation_duration_seconds` - Operation latency by operation, collection
- `db_errors_total` - Database errors by operation, collection

### Billing and Metering

Per-tenant usage is tracked through:
1. **Prometheus metrics:** Payment metrics include `tenant_id` label
2. **MongoDB transactions:** Complete transaction history with app_id, org_id
3. **gRPC metering interceptor:** `grpc_metering_total` counter from service-core

Query tenant billing:
```promql
# Transactions by tenant and status
sum(payment_transactions_total{tenant_id="acme"}) by (status)

# Total payment amount by tenant
sum(payment_amount_total{tenant_id="acme"}) by (currency)
```

### Tracing
- OpenTelemetry spans for all operations
- Trace ID propagation to Razorpay calls
- Tenant/transaction context in spans
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

## Database Indexes

Compound indexes for tenant-scoped queries:
- `(app_id, org_id, _id)` - Transaction lookup
- `(app_id, org_id, user_id)` - User-scoped queries
- `(app_id, org_id, status)` - Status filtering
- `(provider_order_id)` - Webhook lookups

## Payment Providers

| Provider | Status | Capabilities |
|----------|--------|--------------|
| **Razorpay** | Implemented | Orders, payments, webhooks, refunds |
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
- UPI Specification: https://www.npci.org.in/what-we-do/upi/product-overview
