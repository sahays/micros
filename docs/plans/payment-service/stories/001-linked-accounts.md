# Story: Linked Accounts

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement CreateLinkedAccount, GetLinkedAccount, UpdateLinkedAccount, ListLinkedAccounts, and UpdateCommissionConfig gRPC methods for managing Razorpay Route linked accounts and commission configuration.

## Tasks

- [ ] Define proto messages: LinkedAccount, LinkedAccountStatus enum, CommissionConfig, CommissionType enum
- [ ] Define proto messages: CreateLinkedAccountRequest/Response
- [ ] Define proto messages: GetLinkedAccountRequest/Response
- [ ] Define proto messages: UpdateLinkedAccountRequest/Response
- [ ] Define proto messages: ListLinkedAccountsRequest/Response
- [ ] Define proto messages: UpdateCommissionConfigRequest/Response
- [ ] Add LinkedAccount MongoDB collection and indexes
- [ ] Implement Razorpay Route API client for linked account creation
- [ ] Implement CreateLinkedAccount handler with Razorpay Route integration
- [ ] Implement GetLinkedAccount handler
- [ ] Implement UpdateLinkedAccount handler
- [ ] Implement ListLinkedAccounts handler with filters and pagination
- [ ] Implement UpdateCommissionConfig handler
- [ ] Add capability checks to all methods
- [ ] Add metering for linked account operations
- [ ] Add RAZORPAY_ROUTE_ENABLED feature flag check

## gRPC Methods

### CreateLinkedAccount
**Input:** tenant_id, org_id, business_name, business_type, legal_info (pan, gst), bank_account (account_number, ifsc, beneficiary_name), contact_info (email, phone)
**Output:** linked_account

**Validation:**
- org_id does not already have a linked account (one per org)
- business_name is non-empty
- PAN format is valid (10 alphanumeric)
- Bank account details are complete
- RAZORPAY_ROUTE_ENABLED must be true

**Business Logic:**
- Creates linked account in Razorpay via Route API using platform credentials
- Stores linked account with razorpay_account_id and status CREATED
- Razorpay begins KYC review asynchronously (status updates via webhooks)

**Capability:** `payment.linked_account:create`

### GetLinkedAccount
**Input:** tenant_id, linked_account_id (or org_id)
**Output:** linked_account with current status and commission config

**Capability:** `payment.linked_account:read`

### UpdateLinkedAccount
**Input:** tenant_id, linked_account_id, business_name, legal_info, bank_account, contact_info
**Output:** linked_account

**Validation:**
- Linked account exists
- Only updatable fields are modified (cannot change org_id or razorpay_account_id)

**Business Logic:**
- Updates linked account details in Razorpay via Route API
- Updates local record

**Capability:** `payment.linked_account:update`

### ListLinkedAccounts
**Input:** tenant_id, status (optional), page_size, page_token
**Output:** linked_accounts[], next_page_token

**Capability:** `payment.linked_account:read`

### UpdateCommissionConfig
**Input:** tenant_id, linked_account_id, commission_type (percentage, fixed, both), percentage_value, fixed_value, currency
**Output:** linked_account with updated commission config

**Validation:**
- Linked account exists
- Commission type is valid enum
- percentage_value is 0-10000 (0-100.00% in basis points)
- fixed_value is non-negative if type is fixed or both

**Business Logic:**
- Updates commission config on local linked account record
- Commission applied on subsequent transfers

**Capability:** `payment.commission:manage`

## Linked Account Status

| Status | Description |
|--------|-------------|
| `CREATED` | Account created, submitted to Razorpay |
| `UNDER_REVIEW` | Razorpay KYC review in progress |
| `NEEDS_CLARIFICATION` | Razorpay requires additional documents |
| `ACTIVATED` | Account verified, can accept payments |
| `SUSPENDED` | Account suspended |

## Metering

Record on each operation:
```rust
record_linked_account(&tenant_id, "created");
record_linked_account(&tenant_id, &status.to_string());
record_commission_config_update(&tenant_id);
```

## Acceptance Criteria

- [ ] CreateLinkedAccount creates Razorpay linked account via Route API
- [ ] CreateLinkedAccount stores linked account with razorpay_account_id
- [ ] CreateLinkedAccount enforces one linked account per org_id
- [ ] CreateLinkedAccount returns AlreadyExists for duplicate org_id
- [ ] CreateLinkedAccount validates business details
- [ ] CreateLinkedAccount checks RAZORPAY_ROUTE_ENABLED flag
- [ ] GetLinkedAccount returns linked account with commission config
- [ ] GetLinkedAccount returns NOT_FOUND for missing account
- [ ] UpdateLinkedAccount updates details in Razorpay and locally
- [ ] ListLinkedAccounts filters by status
- [ ] ListLinkedAccounts pagination works correctly
- [ ] UpdateCommissionConfig updates commission for linked account
- [ ] UpdateCommissionConfig validates percentage range
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create linked account with valid data returns linked account
- [ ] Create linked account stores razorpay_account_id
- [ ] Create linked account with duplicate org_id returns ALREADY_EXISTS
- [ ] Create linked account with invalid PAN returns INVALID_ARGUMENT
- [ ] Create linked account with Route disabled returns FAILED_PRECONDITION
- [ ] Get linked account returns complete account with commission config
- [ ] Get linked account by org_id returns account
- [ ] Get linked account returns NOT_FOUND for missing account
- [ ] Update linked account updates business details
- [ ] List linked accounts returns only tenant's accounts
- [ ] List linked accounts filters by status
- [ ] List linked accounts pagination works
- [ ] Update commission config sets percentage commission
- [ ] Update commission config sets fixed commission
- [ ] Update commission config sets combined commission
- [ ] Update commission config validates percentage range
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_tenant_onboarding_creates_linked_account` — Verifies [journey 001](../../../user-journeys/payment-service/001-tenant-onboarding.md) steps 1-3: org created, business details submitted, Razorpay linked account created
- [ ] `journey_tenant_onboarding_commission_config` — Verifies [journey 001](../../../user-journeys/payment-service/001-tenant-onboarding.md) step 7: commission rate configured for tenant
