# Payment Service Implementation Plan

## Overview
This document tracks the development of the `payment-service` based on GitHub Issues (source of truth).

---

## Epics

### 1. Payment Service Foundation (Epic #234) ✅ CLOSED
**Overview:** Initialize the payment-service with the standard micros architecture, including database connections, configuration management, and observability hooks.

**Goals:**
1. Establish a runnable Rust service using Axum.
2. Connect to MongoDB for data persistence.
3. Integrate with service-core for logging and error handling.

#### Stories
- [x] **Story #235:** Service Skeleton & Configuration
  - [x] Task #236: Initialize new Cargo project
  - [x] Task #237: Implement Settings struct
  - [x] Task #238: Create basic Axum router
  - [x] Task #239: Add Dockerfile
- [x] **Story #240:** Database Layer & Models
  - [x] Task #241: Define Transaction struct
  - [x] Task #242: Implement MongoRepository
  - [x] Task #243: Create database indexes
- [x] **Story #244:** Observability & Security Middleware
  - [x] Task #245: Integrate tracing-subscriber
  - [x] Task #246: Apply auth_middleware

---

### 2. UPI & QR Code Payments (Epic #258) ✅ CLOSED
**Overview:** Enable direct UPI payments via QR codes.

**Goals:** Generate standard UPI QR strings and images for users to scan.

#### Stories
- [x] **Story #259:** QR Code Generation
  - [x] Task #260: Add qrcodegen crate
  - [x] Task #261: UPI URL logic
  - [x] Task #262: QR Handler

---

### 3. Razorpay Integration (Epic #247) ✅ CLOSED
**Overview:** Implement core payment flows using Razorpay as the provider.

**Goals:**
1. Create Orders to initiate payments.
2. Verify payment success via signatures.
3. Handle Webhooks for asynchronous updates.

#### Stories
- [x] **Story #248:** Order Creation API
  - [x] Task #249: Implement Razorpay Client
  - [x] Task #250: Create handler create_order
  - [x] Task #251: Add unit tests
- [x] **Story #252:** Payment Verification
  - [x] Task #253: Implement signature verification
  - [x] Task #254: Create handler verify_payment
- [x] **Story #255:** Webhook Handling
  - [x] Task #256: Create webhook handler
  - [x] Task #257: Implement event dispatcher

---

### 4. Transaction History & API (Epic #230) ⏳ OPEN
**Overview:** APIs for other services to query payment status and history.

#### Stories
- [ ] **Story #264:** Transaction History API
  - [ ] Task #265: Pagination logic
  - [ ] Task #266: History handlers
- [ ] **Story #267:** Idempotency
  - [ ] Task #268: Idempotency middleware

---

### 5. Multi-Tenancy for SaaS Readiness (Epic #269) 🔄 PARTIAL
**Overview:** Make payment-service SaaS-ready with full multi-tenancy support at both application and organization levels.

**Goals:**
1. Isolate payment data by tenant (app + org hierarchy)
2. Support tenant-specific payment provider configurations
3. Enable tenant-scoped rate limiting and quotas
4. Provide tenant admin APIs for configuration management

**Tenant Model:**
```
App (app_id) ← Registered BFF/Service Client (e.g., secure-frontend, mobile-app)
  └── Organization (org_id) ← Customer/Tenant using the app
       └── Users
       └── Payment Configurations (Razorpay keys, UPI VPA)
       └── Transactions
```

**`app_id`** = The registered service client from auth-service (maps to `client_id` in auth-service's client registration).

**`org_id`** = The customer organization/tenant within that app.

#### Stories
- [x] **Story #286:** Tenant Data Model & Isolation
  - [x] Add `app_id` and `org_id` to `Transaction` model
  - [x] Create compound indexes for tenant isolation
  - [x] Implement tenant context middleware
- [ ] **Story #287:** Tenant-Scoped Payment Providers
  - [ ] Create `PaymentProviderConfig` model
  - [ ] Implement provider config CRUD repository
  - [ ] Refactor Razorpay client for tenant credentials
  - [ ] Refactor UPI QR for tenant VPA
  - [ ] Add credential encryption at rest (AES-256-GCM)
- [ ] **Story #288:** Tenant Administration API
  - [ ] `POST /admin/orgs` (create org under app)
  - [ ] `GET /admin/orgs` (list orgs for app)
  - [ ] `PUT /admin/orgs/{org_id}/providers` (configure provider)
  - [ ] `GET /admin/orgs/{org_id}/providers` (get config, masked)
- [ ] **Story #289:** Tenant Rate Limiting & Quotas
  - [ ] Define quota model
  - [ ] Implement per-org rate limiting using Redis
  - [ ] Add quota enforcement middleware
  - [ ] Create quota usage tracking and alerts
- [ ] **Story #290:** Tenant Data Access Controls
  - [x] Implement row-level security in all queries
  - [x] Add tenant validation to all handlers
  - [ ] Create audit logging for cross-tenant access attempts
  - [ ] Add integration tests for tenant isolation

---

## Backlog / Cleanup
- ~~#211: Task: Initialize new Cargo project (Duplicate of #236)~~ - Closed
- ~~#263: Epic: Transaction History & API (Duplicate of #230)~~ - Closed
