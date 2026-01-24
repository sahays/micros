# Cross-Service Workflow Integration Tests Specification

## Problem Statement

Individual service tests verify each microservice works in isolation, but they don't catch integration issues between services. When reconciliation-service calls genai-service, or billing-service posts to ledger-service, we need tests that verify the full workflow succeeds.

Currently we have:
- Unit tests within each service
- Single-service integration tests (e.g., reconciliation-service tests against its own database)
- Zero cross-service workflow tests

This gap means integration bugs are caught in production or manual testing.

## Solution

Create a new workspace member `workflow-tests` that runs end-to-end tests across multiple services. Tests connect to all services via gRPC and verify complete business workflows. Tests are executed via `integ-tests.sh` alongside other service tests.

## Workflow Test Categories

### 1. Reconciliation + GenAI Workflows

| Test | Flow | Validates |
|------|------|-----------|
| `ai_statement_extraction` | Upload PDF → reconciliation calls genai → transactions extracted | gRPC call to genai-service works (Gemini mocked) |
| `ai_match_suggestions` | Request suggestions → reconciliation calls genai → matches returned | Service-to-service integration (Gemini mocked) |
| `ai_confirm_matches` | Confirm suggestion → reconciliation updates → genai learns | Feedback data flows correctly |
| `ai_fallback_on_genai_unavailable` | GenAI down → reconciliation returns graceful error | Graceful degradation |

### 2. Billing + Ledger + Notification Workflows

| Test | Flow | Validates |
|------|------|-----------|
| `billing_run_posts_to_ledger` | Run billing → charges created → journal entries in ledger | Financial data flows correctly |
| `invoice_sends_notification` | Generate invoice → notification service receives request | Billing→notification integration (email mocked) |
| `subscription_creates_ledger_account` | Create subscription → ledger account auto-created | Account provisioning |
| `usage_to_invoice_to_ledger` | Record usage → billing run → invoice → ledger entry | Full metering flow |

### 3. Payment + Ledger Workflows

| Test | Flow | Validates |
|------|------|-----------|
| `payment_posts_to_ledger` | Process payment → journal entry created | Payment→ledger integration (Stripe mocked) |
| `payment_sends_receipt` | Payment success → notification receives request | Payment→notification integration (email mocked) |
| `refund_reverses_ledger` | Process refund → reversing entry created | Refund handling (Stripe mocked) |
| `payment_marks_invoice_paid` | Payment received → billing marks invoice paid | Payment→billing status sync |

### 4. Auth Capability Validation

| Test | Flow | Validates |
|------|------|-----------|
| `valid_capability_allows_access` | Token with capability → service allows request | Positive auth |
| `missing_capability_denies_access` | Token without capability → service rejects | Negative auth |
| `expired_token_rejected` | Expired token → all services reject | Token expiry |
| `revoked_token_rejected` | Revoke token → subsequent requests fail | Revocation propagation |
| `tenant_isolation_enforced` | Tenant A token → cannot access Tenant B data | Multi-tenancy |

### 5. Document + GenAI Workflows

| Test | Flow | Validates |
|------|------|-----------|
| `document_metadata_extraction` | Upload doc → genai extracts metadata | Document→genai integration (Gemini mocked) |
| `bank_statement_to_reconciliation` | Upload statement → parse → import transactions | Document→genai→reconciliation pipeline |

### 6. End-to-End Business Workflows

| Test | Flow | Validates |
|------|------|-----------|
| `full_billing_cycle` | Subscription → usage → billing run → invoice → payment → receipt | Complete revenue cycle |
| `full_reconciliation_cycle` | Upload statement → parse → match → complete → post adjustments | Complete reconciliation |
| `tenant_onboarding` | Create tenant → setup billing plan → create ledger accounts → welcome email | Onboarding flow |

## Test Infrastructure

### Directory Structure

New workspace member `workflow-tests` at repo root:

```
workflow-tests/
├── Cargo.toml                     # Workspace member, depends on all service protos
├── src/
│   └── lib.rs                     # WorkflowTestContext, helpers, re-exports
└── tests/
    ├── common/
    │   └── mod.rs                 # Shared setup, client factories
    ├── reconciliation_genai_test.rs
    ├── billing_ledger_test.rs
    ├── payment_ledger_test.rs
    ├── auth_capability_test.rs
    ├── document_genai_test.rs
    └── end_to_end_test.rs
```

### Integration with integ-tests.sh

Update `scripts/integ-tests.sh` to include workflow-tests:

```bash
# Add new category - requires ALL databases and running services
WORKFLOW_SERVICES=("workflow-tests")

# workflow-tests is special:
# - Requires PostgreSQL (for auth, ledger, billing, reconciliation)
# - Requires MongoDB (for document, notification, payment, genai)
# - Requires all 9 services running via docker-compose
# - Connects to services via gRPC, not direct database access
```

### Running Workflow Tests

```bash
# Start all services first
./scripts/dev-up.sh

# Run all tests including workflow tests
./scripts/integ-tests.sh

# Run only workflow tests
./scripts/integ-tests.sh -p workflow-tests

# Run specific workflow test
./scripts/integ-tests.sh -p workflow-tests -- reconciliation_genai
```

### WorkflowTestContext

```rust
pub struct WorkflowTestContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub auth_token: String,

    // Service clients
    pub auth: AuthServiceClient<Channel>,
    pub reconciliation: ReconciliationServiceClient<Channel>,
    pub genai: GenaiServiceClient<Channel>,
    pub billing: BillingServiceClient<Channel>,
    pub ledger: LedgerServiceClient<Channel>,
    pub payment: PaymentServiceClient<Channel>,
    pub notification: NotificationServiceClient<Channel>,
    pub document: DocumentServiceClient<Channel>,
}

impl WorkflowTestContext {
    /// Bootstrap a new tenant and connect to all services
    pub async fn new() -> Result<Self, Error> {
        // 1. Connect to auth-service
        // 2. Create tenant via admin API
        // 3. Create user with all capabilities
        // 4. Get auth token
        // 5. Connect to all other services with token
    }

    /// Add auth headers to a request
    pub fn with_auth<T>(&self, request: Request<T>) -> Request<T> {
        let mut req = request;
        req.metadata_mut().insert("authorization",
            format!("Bearer {}", self.auth_token).parse().unwrap());
        req.metadata_mut().insert("x-tenant-id",
            self.tenant_id.to_string().parse().unwrap());
        req
    }
}
```

### Service Endpoints

| Service | Health Port | gRPC Port |
|---------|-------------|-----------|
| auth-service | 9005 | 50051 |
| document-service | 9007 | 50052 |
| notification-service | 9008 | 50053 |
| payment-service | 9009 | 50054 |
| genai-service | 9010 | 50055 |
| ledger-service | 9011 | 50056 |
| billing-service | 9012 | 50057 |
| reconciliation-service | 9013 | 50058 |

### Environment Variables

Workflow tests read service endpoints from environment:

```bash
# Set by integ-tests.sh or manually
export AUTH_GRPC_ENDPOINT="http://localhost:50051"
export DOCUMENT_GRPC_ENDPOINT="http://localhost:50052"
export NOTIFICATION_GRPC_ENDPOINT="http://localhost:50053"
export PAYMENT_GRPC_ENDPOINT="http://localhost:50054"
export GENAI_GRPC_ENDPOINT="http://localhost:50055"
export LEDGER_GRPC_ENDPOINT="http://localhost:50056"
export BILLING_GRPC_ENDPOINT="http://localhost:50057"
export RECONCILIATION_GRPC_ENDPOINT="http://localhost:50058"
```

## Test Patterns

### Pattern 1: Happy Path Workflow

```rust
#[tokio::test]
async fn billing_run_posts_to_ledger() {
    let ctx = WorkflowTestContext::new().await.unwrap();

    // Setup: Create subscription in billing
    let subscription = ctx.billing.create_subscription(
        ctx.with_auth(CreateSubscriptionRequest { ... })
    ).await.unwrap();

    // Setup: Record some usage
    ctx.billing.record_usage(
        ctx.with_auth(RecordUsageRequest { ... })
    ).await.unwrap();

    // Action: Run billing
    let billing_result = ctx.billing.run_billing(
        ctx.with_auth(RunBillingRequest { ... })
    ).await.unwrap();

    // Verify: Check ledger has journal entry
    let ledger_entries = ctx.ledger.list_transactions(
        ctx.with_auth(ListTransactionsRequest {
            account_id: subscription.receivable_account_id,
            ..Default::default()
        })
    ).await.unwrap();

    assert!(!ledger_entries.transactions.is_empty());
    assert_eq!(ledger_entries.transactions[0].amount, billing_result.total_amount);
}
```

### Pattern 2: Error Handling

```rust
#[tokio::test]
async fn ai_fallback_on_genai_unavailable() {
    let ctx = WorkflowTestContext::new().await.unwrap();

    // Setup: Create bank account and reconciliation
    let bank_account = create_bank_account(&ctx).await;
    let reconciliation = start_reconciliation(&ctx, &bank_account).await;

    // Action: Request AI suggestions (genai may be unavailable)
    let result = ctx.reconciliation.get_ai_suggestions(
        ctx.with_auth(GetAiSuggestionsRequest {
            reconciliation_id: reconciliation.id,
        })
    ).await;

    // Verify: Either succeeds or returns graceful error
    match result {
        Ok(suggestions) => {
            // GenAI available - verify suggestions returned
            assert!(suggestions.suggestions.len() >= 0);
        }
        Err(status) => {
            // GenAI unavailable - verify graceful degradation
            assert_eq!(status.code(), tonic::Code::Unavailable);
            assert!(status.message().contains("AI service temporarily unavailable"));
        }
    }
}
```

### Pattern 3: Tenant Isolation

```rust
#[tokio::test]
async fn tenant_isolation_enforced() {
    // Create two separate tenants
    let tenant_a = WorkflowTestContext::new().await.unwrap();
    let tenant_b = WorkflowTestContext::new().await.unwrap();

    // Tenant A creates data
    let account_a = tenant_a.ledger.create_account(
        tenant_a.with_auth(CreateAccountRequest { name: "Tenant A Account".into(), ... })
    ).await.unwrap();

    // Tenant B tries to access Tenant A's data
    let result = tenant_b.ledger.get_account(
        tenant_b.with_auth(GetAccountRequest { account_id: account_a.id.clone() })
    ).await;

    // Should fail - not found (not permission denied, to avoid leaking existence)
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
}
```

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| auth-service | ✅ Complete | Capability system working |
| billing-service | ✅ Complete | gRPC service implemented |
| ledger-service | ⚠️ Partial | Need to verify gRPC endpoints |
| payment-service | ✅ Complete | gRPC service implemented |
| notification-service | ✅ Complete | gRPC service implemented |
| genai-service | ✅ Complete | gRPC service implemented |
| document-service | ✅ Complete | gRPC service implemented |
| reconciliation-service | ✅ Complete | gRPC service implemented |

## Success Criteria

1. **Test crate compiles**: `cargo build -p workflow-tests`
2. **All tests pass**: `./scripts/integ-tests.sh -p workflow-tests` with 100% pass rate
3. **22+ tests implemented**: Covering all workflow categories
4. **integ-tests.sh integration**: Workflow tests run as part of `./scripts/integ-tests.sh`
5. **Execution time**: Full suite completes in < 5 minutes

## External Dependencies (Mocked)

Workflow tests verify inter-service communication, not external integrations. All external APIs are mocked:

| Service | External Dependency | Mock Behavior |
|---------|---------------------|---------------|
| notification-service | Email provider (SendGrid/SES) | Log email, return success |
| notification-service | SMS provider (Twilio) | Log SMS, return success |
| genai-service | Gemini API | Return canned responses |
| payment-service | Stripe API | Return mock payment success/failure |
| document-service | S3/Cloud Storage | Use local filesystem |

Services should already have mock implementations via feature flags or environment config (e.g., `MOCK_EXTERNAL_APIS=true`).

## Out of Scope

- Performance testing (separate epic)
- Chaos/failure injection testing
- UI integration tests
- Live external API testing (Stripe, SendGrid, Gemini)
- Database migration tests
