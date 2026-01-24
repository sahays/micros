# Plan: Cross-Service Workflow Integration Tests

**Spec:** `docs/specs/workflow-integration-tests.md`

## Overview

Implement `workflow-tests` workspace member with 22 cross-service integration tests. Tests connect to running services via gRPC and verify end-to-end business workflows with mocked external dependencies.

---

## Phase 1: Infrastructure Setup

### Story 001: Create workflow-tests Crate

Create the workspace member with test utilities and gRPC client connections.

**Tasks:**

- [ ] **Task 1.1**: Create `workflow-tests/Cargo.toml`
  - Add as workspace member in root `Cargo.toml`
  - Dependencies: tokio, tonic, prost, uuid, all service proto crates
  - Dev-dependencies: serial_test

- [ ] **Task 1.2**: Create `workflow-tests/src/lib.rs`
  - `WorkflowTestContext` struct with all service clients
  - `new()` - bootstrap tenant, get auth token, connect all clients
  - `with_auth()` - add auth headers to requests
  - Service endpoint constants from env vars

- [ ] **Task 1.3**: Create `workflow-tests/tests/common/mod.rs`
  - Re-export `WorkflowTestContext`
  - Helper functions: `create_tenant()`, `create_user_with_capabilities()`
  - Test data builders for common entities

- [ ] **Task 1.4**: Update `scripts/integ-tests.sh`
  - Add `workflow-tests` to `WORKFLOW_SERVICES` array
  - Require both PostgreSQL and MongoDB
  - Check all services are running before workflow tests
  - Export gRPC endpoint environment variables

- [ ] **Task 1.5**: Add health check utility
  - `wait_for_services()` - poll health endpoints until all services ready
  - Timeout after 60 seconds with clear error message

**Acceptance:**
- `cargo build -p workflow-tests` compiles
- `./scripts/integ-tests.sh -p workflow-tests` runs (with 0 tests initially)

---

## Phase 2: Auth Capability Tests

### Story 002: Auth Capability Validation Tests

Test that capability-based authorization works across all services.

**Tasks:**

- [ ] **Task 2.1**: Create `workflow-tests/tests/auth_capability_test.rs`

- [ ] **Task 2.2**: Implement `valid_capability_allows_access`
  - Create user with `ledger.account:create` capability
  - Call ledger-service create account
  - Assert: succeeds

- [ ] **Task 2.3**: Implement `missing_capability_denies_access`
  - Create user WITHOUT `ledger.account:create` capability
  - Call ledger-service create account
  - Assert: returns `PermissionDenied`

- [ ] **Task 2.4**: Implement `expired_token_rejected`
  - Get token, wait for expiry (or use short-lived test token)
  - Call any service
  - Assert: returns `Unauthenticated`

- [ ] **Task 2.5**: Implement `revoked_token_rejected`
  - Get token, call logout/revoke endpoint
  - Call any service with revoked token
  - Assert: returns `Unauthenticated`

- [ ] **Task 2.6**: Implement `tenant_isolation_enforced`
  - Create Tenant A with data
  - Create Tenant B
  - Tenant B tries to access Tenant A's data
  - Assert: returns `NotFound` (not `PermissionDenied`)

**Acceptance:**
- 5 tests pass
- Tests run in < 30 seconds

---

## Phase 3: Reconciliation + GenAI Tests

### Story 003: Reconciliation + GenAI Workflow Tests

Test reconciliation service calling genai service for AI-powered features.

**Tasks:**

- [ ] **Task 3.1**: Create `workflow-tests/tests/reconciliation_genai_test.rs`

- [ ] **Task 3.2**: Implement `ai_statement_extraction`
  - Create bank account
  - Import statement with raw transaction data
  - Verify: reconciliation called genai, transactions parsed correctly
  - Note: Gemini mocked, returns canned parsed transactions

- [ ] **Task 3.3**: Implement `ai_match_suggestions`
  - Setup: bank account, reconciliation, unmatched transactions
  - Call get AI suggestions
  - Verify: suggestions returned from genai service
  - Note: Gemini mocked, returns canned suggestions

- [ ] **Task 3.4**: Implement `ai_confirm_matches`
  - Setup: get AI suggestions
  - Confirm a suggestion
  - Verify: match created, feedback sent to genai

- [ ] **Task 3.5**: Implement `ai_fallback_on_genai_unavailable`
  - Note: May need to skip if genai always available in test env
  - Or: test with invalid genai endpoint
  - Verify: graceful error returned, not 500

**Acceptance:**
- 4 tests pass (or 3 + 1 skipped)
- Tests verify gRPC calls between services

---

## Phase 4: Billing + Ledger + Notification Tests

### Story 004: Billing + Ledger + Notification Workflow Tests

Test the billing cycle posting to ledger and triggering notifications.

**Tasks:**

- [ ] **Task 4.1**: Create `workflow-tests/tests/billing_ledger_test.rs`

- [ ] **Task 4.2**: Implement `billing_run_posts_to_ledger`
  - Create billing plan, subscription
  - Record usage
  - Run billing cycle
  - Verify: journal entry created in ledger-service

- [ ] **Task 4.3**: Implement `invoice_sends_notification`
  - Create subscription, run billing
  - Verify: notification-service received email request
  - Note: Email delivery mocked

- [ ] **Task 4.4**: Implement `subscription_creates_ledger_account`
  - Create new subscription
  - Verify: receivable account auto-created in ledger

- [ ] **Task 4.5**: Implement `usage_to_invoice_to_ledger`
  - Full flow: record usage → billing run → invoice generated → ledger entry
  - Verify: amounts match throughout

**Acceptance:**
- 4 tests pass
- Tests verify billing→ledger and billing→notification integration

---

## Phase 5: Payment + Ledger Tests

### Story 005: Payment + Ledger Workflow Tests

Test payment processing posting to ledger and triggering receipts.

**Tasks:**

- [ ] **Task 5.1**: Create `workflow-tests/tests/payment_ledger_test.rs`

- [ ] **Task 5.2**: Implement `payment_posts_to_ledger`
  - Process a payment (Stripe mocked)
  - Verify: journal entry created in ledger-service

- [ ] **Task 5.3**: Implement `payment_sends_receipt`
  - Process a payment
  - Verify: notification-service received receipt email request

- [ ] **Task 5.4**: Implement `refund_reverses_ledger`
  - Process payment, then refund
  - Verify: reversing journal entry in ledger

- [ ] **Task 5.5**: Implement `payment_marks_invoice_paid`
  - Create invoice via billing
  - Process payment for that invoice
  - Verify: invoice status updated to paid in billing-service

**Acceptance:**
- 4 tests pass
- Tests verify payment→ledger and payment→notification integration

---

## Phase 6: Document + GenAI Tests

### Story 006: Document + GenAI Workflow Tests

Test document processing with AI metadata extraction.

**Tasks:**

- [ ] **Task 6.1**: Create `workflow-tests/tests/document_genai_test.rs`

- [ ] **Task 6.2**: Implement `document_metadata_extraction`
  - Upload a document
  - Verify: genai-service called for metadata extraction
  - Verify: metadata stored with document

- [ ] **Task 6.3**: Implement `bank_statement_to_reconciliation`
  - Upload bank statement PDF
  - Call parse endpoint (document → genai)
  - Import parsed transactions to reconciliation
  - Verify: transactions appear in reconciliation-service

**Acceptance:**
- 2 tests pass
- Tests verify document→genai pipeline

---

## Phase 7: End-to-End Business Workflows

### Story 007: End-to-End Business Workflow Tests

Test complete business processes spanning multiple services.

**Tasks:**

- [ ] **Task 7.1**: Create `workflow-tests/tests/end_to_end_test.rs`

- [ ] **Task 7.2**: Implement `full_billing_cycle`
  - Create tenant, billing plan, subscription
  - Record usage over time
  - Run billing cycle → invoice generated
  - Process payment for invoice
  - Verify: receipt notification sent
  - Verify: all ledger entries balanced

- [ ] **Task 7.3**: Implement `full_reconciliation_cycle`
  - Create tenant, bank account, ledger account
  - Upload bank statement
  - Parse with genai
  - Import transactions
  - Get AI match suggestions
  - Confirm matches
  - Complete reconciliation
  - Verify: adjustments posted to ledger

- [ ] **Task 7.4**: Implement `tenant_onboarding`
  - Create new tenant via auth-service
  - Setup default billing plan
  - Create default ledger accounts (receivable, revenue)
  - Send welcome notification
  - Verify: all entities created, notification sent

**Acceptance:**
- 3 tests pass
- Each test exercises 3+ services

---

## Summary

| Phase | Story | Tests | Services Covered |
|-------|-------|-------|------------------|
| 1 | Infrastructure | 0 | Setup only |
| 2 | Auth Capability | 5 | auth → all |
| 3 | Reconciliation + GenAI | 4 | reconciliation ↔ genai |
| 4 | Billing + Ledger + Notification | 4 | billing → ledger, notification |
| 5 | Payment + Ledger | 4 | payment → ledger, billing, notification |
| 6 | Document + GenAI | 2 | document → genai → reconciliation |
| 7 | End-to-End | 3 | All services |
| **Total** | **7 stories** | **22 tests** | |

---

## Execution Order

1. **Phase 1** first - creates the test infrastructure
2. **Phase 2** next - auth tests are foundation for all other tests
3. **Phases 3-6** can be done in parallel (independent service pairs)
4. **Phase 7** last - depends on all other phases working

---

## Dependencies

| Blocker | Status | Notes |
|---------|--------|-------|
| All services have gRPC endpoints | ✅ | Implemented |
| Services have mock mode for external APIs | ⚠️ | May need to add |
| integ-tests.sh supports workflow-tests | ❌ | Task 1.4 |
| Auth admin API for tenant creation | ✅ | Exists |

---

## Verification

After all phases complete:

```bash
# Start all services
./scripts/dev-up.sh

# Run full test suite
./scripts/integ-tests.sh

# Expected output: 22 workflow tests pass
```
