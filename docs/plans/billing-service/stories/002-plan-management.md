# Story: Plan Management

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement CreatePlan, GetPlan, UpdatePlan, ListPlans, and ArchivePlan gRPC methods for billing plan management. Plans define pricing templates with optional usage-based components.

## Tasks

- [ ] Define proto messages: BillingPlan, UsageComponent
- [ ] Define proto messages: CreatePlanRequest/Response
- [ ] Define proto messages: GetPlanRequest/Response
- [ ] Define proto messages: UpdatePlanRequest/Response
- [ ] Define proto messages: ListPlansRequest/Response
- [ ] Define proto messages: ArchivePlanRequest/Response
- [ ] Implement CreatePlan handler with validation
- [ ] Implement GetPlan handler
- [ ] Implement UpdatePlan handler (active plans only)
- [ ] Implement ListPlans handler with filters and pagination
- [ ] Implement ArchivePlan handler
- [ ] Add usage component CRUD within plan
- [ ] Add capability checks to all methods

## gRPC Methods

### CreatePlan
**Input:** tenant_id, name, description, billing_interval, interval_count, base_price, currency, tax_rate_id, usage_components[], metadata
**Output:** plan

**Validation:**
- name is non-empty
- billing_interval is valid enum
- base_price >= 0
- currency is valid (3-char ISO code)

**Capability:** `billing.plan:create`

### GetPlan
**Input:** tenant_id, plan_id
**Output:** plan with usage_components

**Capability:** `billing.plan:read`

### UpdatePlan
**Input:** tenant_id, plan_id, name, description, base_price, tax_rate_id, metadata
**Output:** plan

**Validation:**
- Plan must not be archived
- Cannot change billing_interval or currency (would affect existing subscriptions)

**Capability:** `billing.plan:update`

### ListPlans
**Input:** tenant_id, include_archived, page_size, page_token
**Output:** plans[], next_page_token

**Capability:** `billing.plan:read`

### ArchivePlan
**Input:** tenant_id, plan_id
**Output:** plan

**Validation:**
- Archived plans cannot accept new subscriptions
- Existing subscriptions continue on archived plan

**Capability:** `billing.plan:update`

## Usage Components

### AddUsageComponent
**Input:** plan_id, name, unit_name, unit_price, included_units
**Output:** component

### UpdateUsageComponent
**Input:** component_id, name, unit_name, unit_price, included_units
**Output:** component

### RemoveUsageComponent
**Input:** component_id
**Output:** success

**Note:** Components are managed as part of plan operations, not separate RPCs.

## Metering

Record on each operation:
```rust
record_plan_operation(&tenant_id, "created");
record_plan_operation(&tenant_id, "updated");
record_plan_operation(&tenant_id, "archived");
```

## Acceptance Criteria

- [ ] CreatePlan returns new plan with components
- [ ] GetPlan returns plan with all usage components
- [ ] GetPlan returns NOT_FOUND for missing plan
- [ ] UpdatePlan modifies active plan
- [ ] UpdatePlan rejects archived plan modifications
- [ ] ListPlans filters by include_archived
- [ ] ListPlans pagination works correctly
- [ ] ArchivePlan marks plan as archived
- [ ] Archived plans still queryable
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create plan with valid data returns plan
- [ ] Create plan with usage components
- [ ] Get plan returns complete plan with components
- [ ] Update active plan succeeds
- [ ] Update archived plan returns FAILED_PRECONDITION
- [ ] Archive plan succeeds
- [ ] List plans returns only tenant's plans
- [ ] List plans with include_archived shows archived
- [ ] List plans pagination works
- [ ] Operations without capability return PERMISSION_DENIED
