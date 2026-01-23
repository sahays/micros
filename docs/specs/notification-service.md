# Notification Service

**Multi-channel notification delivery with email, SMS, and push support.**

## Problem

Applications need to send notifications across multiple channels (email, SMS, push). Building custom integrations per app leads to:
- Inconsistent provider abstractions
- No centralized delivery tracking or audit trails
- No multi-tenant isolation
- Duplicated provider configuration and error handling

## Solution

A reusable notification microservice providing:
- Multi-channel delivery (email, SMS, push notifications)
- Provider abstraction (SMTP, Msg91, FCM)
- Centralized notification tracking
- Per-tenant isolation via metadata
- Batch notification support
- Mock providers for development/testing

## Core Principles

- **Multi-tenant:** Isolation via metadata fields (tenant_id, user_id)
- **Provider abstraction:** Pluggable providers per channel
- **Resilient:** Graceful fallback to mock providers when disabled
- **Auditable:** Complete notification history with status tracking
- **BFF trust model:** Trusts upstream services for authorization

## Data Model

### Notification

| Field | Type | Description |
|-------|------|-------------|
| `notification_id` | UUID | Unique identifier |
| `channel` | Enum | `Email`, `Sms`, `Push` |
| `status` | Enum | `Queued`, `Sent`, `Delivered`, `Failed` |
| `recipient` | String | Email/phone/device token |
| `subject` | String | Email subject (optional) |
| `body` | String | Plain text content |
| `body_html` | String | HTML content (email only) |
| `from_name` | String | Sender display name |
| `reply_to` | String | Reply-to address |
| `platform` | Enum | `Fcm`, `Apns` (push only) |
| `push_title` | String | Push notification title |
| `push_data` | Map | Custom data payload (push only) |
| `metadata` | Map | Custom tracking (tenant_id, user_id, etc.) |
| `provider_id` | String | External provider message ID |
| `error_message` | String | Failure reason if failed |
| `created_utc` | Timestamp | Creation time |
| `sent_utc` | Timestamp | When sent (optional) |
| `delivered_utc` | Timestamp | When delivered (optional) |
| `failed_utc` | Timestamp | When failed (optional) |

### Status Transitions

```
Queued → Sent → Delivered
   ↓       ↓
Failed  Failed
```

## gRPC Service: NotificationService

| Method | Type | Description |
|--------|------|-------------|
| `SendEmail` | Unary | Send email notification |
| `SendSms` | Unary | Send SMS notification |
| `SendPush` | Unary | Send push notification |
| `SendBatch` | Unary | Send up to 100 notifications |
| `GetNotification` | Unary | Retrieve notification by ID |
| `ListNotifications` | Unary | List notifications with filters |

## Providers

### Email (SMTP)

- Provider: `SmtpProvider` via lettre crate
- Configuration: `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`
- Features: HTML/text multipart, custom from name, reply-to
- Fallback: `MockEmailProvider` when disabled

### SMS (Msg91)

- Provider: `Msg91Provider` via HTTP API
- Configuration: `MSG91_AUTH_KEY`, `MSG91_SENDER_ID`
- Endpoint: `https://api.msg91.com/api/v5/flow/`
- Fallback: `MockSmsProvider` when disabled

### Push (FCM)

- Provider: `FcmProvider` via Firebase Cloud Messaging
- Configuration: `FCM_PROJECT_ID`, `FCM_SERVICE_ACCOUNT_KEY`
- Platforms: Android (FCM), iOS (APNS)
- Fallback: `MockPushProvider` when disabled

## Authentication Model

### Request Metadata

All requests support tenant context via metadata:
- `tenant_id`: Required for tenant-scoped operations
- `user_id`: Optional user identifier

### Trust Model

Notification-service uses a **BFF trust model**:
- Trusts upstream services to validate authorization
- Does NOT validate JWT tokens directly
- Multi-tenant isolation via metadata in database queries

## Capabilities

Capabilities control access to notification-service operations.

**Format:** `{domain}.{resource}:{action}`

| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `notification.email:send` | SendEmail | Send emails |
| `notification.sms:send` | SendSms | Send SMS messages |
| `notification.push:send` | SendPush | Send push notifications |
| `notification.batch:send` | SendBatch | Send batch notifications |
| `notification:read` | GetNotification, ListNotifications | View notifications |

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
    │ 2. Check notification:* capabilities
    │ 3. Add tenant metadata
    ▼
Notification Service
    │
    │ Trust caller, process request
    ▼
Email/SMS/Push Providers + MongoDB
```

## Use Cases

- **Transactional emails:** Password reset, verification, receipts
- **Marketing SMS:** Promotions, alerts, reminders
- **Push notifications:** Real-time updates, engagement
- **Batch notifications:** Bulk campaigns (max 100 per request)
- **Notification history:** Audit trail with status tracking

## Key Features

- **Multi-channel:** Single API for email, SMS, and push
- **Provider abstraction:** Easy to add new providers
- **Graceful degradation:** Mock providers for development
- **Status tracking:** Complete notification lifecycle
- **Batch support:** Up to 100 notifications per request
- **Metadata:** Flexible key-value tracking (tenant_id, user_id)
- **Health endpoints:** HTTP /health, /ready, /metrics
- **gRPC reflection:** Debugging via grpcurl

## Edge Cases

- **Invalid recipient:** Returns InvalidArgument
- **Notification not found:** Returns NotFound
- **Provider not configured:** Falls back to mock, marks as sent
- **Provider failure:** Marks as failed with error message
- **Batch too large:** Returns InvalidArgument (max 100)
- **Missing required fields:** Returns InvalidArgument
- **Database error:** Returns Internal

## Non-Goals

- Direct user authentication (use auth-service)
- Message queuing/scheduling (future enhancement)
- Template management (future enhancement)
- Delivery webhooks from providers (future enhancement)

## Observability

### HTTP Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Liveness probe |
| `GET /ready` | Readiness probe |
| `GET /metrics` | Prometheus metrics |

### Logging

- Structured JSON to stdout (PLG-compatible)
- Notification lifecycle logging with IDs
- Provider call logging with success/failure
- Error logging with context

### Metrics

**gRPC Metrics:**
- `grpc_requests_total` - Requests by method, status
- `grpc_request_duration_seconds` - Duration histogram by method
- `grpc_requests_in_flight` - Current request count by method
- `grpc_metering_total` - Requests by tenant_id (via interceptor)

**Notification Metrics (Per-Tenant Billing):**
- `notification_sent_total{tenant_id, channel, status}` - Notifications by tenant, channel, status
- `notification_provider_calls_total{provider, status}` - Provider API calls

**Database Metrics:**
- `db_operation_duration_seconds` - Operation latency by operation, collection
- `db_errors_total` - Database errors by operation, collection

### Billing and Metering

Per-tenant usage is tracked through:
1. **Prometheus metrics:** Notification metrics include `tenant_id` label
2. **MongoDB notifications:** Complete history with metadata
3. **gRPC metering interceptor:** `grpc_metering_total` counter from service-core

Query tenant billing:
```promql
# Notifications by tenant and channel
sum(notification_sent_total{tenant_id="acme"}) by (channel)

# Email notifications by tenant
notification_sent_total{tenant_id="acme", channel="email"}
```

### Tracing

- OpenTelemetry spans for all operations
- Trace ID propagation to provider calls
- Notification context in spans
- Exports to Tempo via OTLP/gRPC

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `APP_PORT` | HTTP server port | `3000` |
| `MONGODB_URI` | MongoDB connection string | (required) |
| `MONGODB_DATABASE` | Database name | `notification_db` |
| `SMTP_HOST` | SMTP server hostname | `smtp.gmail.com` |
| `SMTP_PORT` | SMTP server port | `587` |
| `SMTP_USER` | SMTP username | (optional) |
| `SMTP_PASSWORD` | SMTP password | (optional) |
| `SMTP_FROM_EMAIL` | Default from email | `noreply@example.com` |
| `SMTP_FROM_NAME` | Default from name | `Notification Service` |
| `SMTP_ENABLED` | Enable SMTP provider | `false` |
| `MSG91_AUTH_KEY` | Msg91 API key | (optional) |
| `MSG91_SENDER_ID` | Msg91 sender ID | (optional) |
| `MSG91_ENABLED` | Enable Msg91 provider | `false` |
| `FCM_PROJECT_ID` | Firebase project ID | (optional) |
| `FCM_SERVICE_ACCOUNT_KEY` | Firebase service account | (optional) |
| `FCM_ENABLED` | Enable FCM provider | `false` |
| `AUTH_SERVICE_ENDPOINT` | Auth-service endpoint (enables capability enforcement) | (unset) |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://tempo:4317` |

## Database Indexes

Indexes for efficient queries:
- `status_idx` - Status filtering
- `channel_idx` - Channel filtering
- `created_utc_idx` - Recent-first sorting (descending)
- `notification_id_idx` - Unique lookup
- `metadata.user_id_idx` - User-scoped queries (sparse)
- `metadata.tenant_id_idx` - Tenant-scoped queries (sparse)

## Implementation Files

| File | Description |
|------|-------------|
| `src/main.rs` | Entry point, initializes metrics and tracing |
| `src/startup.rs` | Application lifecycle, HTTP + gRPC server setup |
| `src/config/mod.rs` | Configuration structs and environment loading |
| `src/grpc/notification_service.rs` | gRPC method implementations |
| `src/grpc/capability_check.rs` | Capability enforcement module |
| `src/services/database.rs` | MongoDB operations |
| `src/services/metrics.rs` | Per-tenant metrics (Prometheus) |
| `src/services/providers/email.rs` | SMTP + mock email provider |
| `src/services/providers/sms.rs` | Msg91 + mock SMS provider |
| `src/services/providers/push.rs` | FCM + mock push provider |
| `src/models/notification.rs` | Data models and conversions |
| `tests/notification_test.rs` | Integration tests |
| `tests/common/mod.rs` | Test setup and helpers |

## References

- Proto Definition: `proto/micros/notification/v1/notification.proto`
- Lettre (SMTP): https://lettre.rs/
- Msg91 API: https://docs.msg91.com/
- Firebase Cloud Messaging: https://firebase.google.com/docs/cloud-messaging
