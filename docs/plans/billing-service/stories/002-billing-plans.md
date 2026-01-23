# Story: Billing Plans

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement CreatePlan, GetPlan, ListPlans, and ArchivePlan gRPC methods for managing billing plan templates.

## Tasks

- [ ] Define proto messages: BillingPlan, UsageComponent
- [ ] Define proto messages: CreatePlanRequest/Response, GetPlanRequest/Response
- [ ] Define proto messages: ListPlansRequest/Response, ArchivePlanRequest/Response
- [ ] Implement CreatePlan handler with usage components
- [ ] Implement GetPlan handler
- [ ] Implement ListPlans handler with filters
- [ ] Implement ArchivePlan handler (soft delete)

## gRPC Methods

### CreatePlan
**Input:** tenant_id, name, description, billing_interval, interval_count, base_price, currency, usage_components[], metadata
**Output:** plan

**Validation:**
- name is not empty
- billing_interval in (daily, weekly, monthly, quarterly, annually)
- interval_count > 0
- base_price >= 0
- currency is valid ISO 4217

### GetPlan
**Input:** tenant_id, plan_id
**Output:** plan with usage_components

### ListPlans
**Input:** tenant_id, is_active (optional), page_size, page_token
**Output:** plans[], next_page_token

### ArchivePlan
**Input:** tenant_id, plan_id
**Output:** plan

**Behavior:**
- Sets is_active = false and archived_utc
- Existing subscriptions continue on archived plan
- Cannot create new subscriptions on archived plan

## Billing Intervals

| Interval | Typical Use |
|----------|-------------|
| daily | API/usage-heavy services |
| weekly | Short-term services |
| monthly | Standard SaaS |
| quarterly | Enterprise contracts |
| annually | Discounted annual plans |

## Usage Components

Plans can have zero or more usage components for metered billing:
- component_id, name, unit_name
- unit_price (per unit over included)
- included_units (free tier)

Example: "Pro Plan" with 1000 included API calls, $0.001 per additional call

## Acceptance Criteria

- [ ] CreatePlan creates plan with base pricing
- [ ] CreatePlan creates plan with usage components
- [ ] GetPlan returns plan with all components
- [ ] ListPlans filters by active status
- [ ] ArchivePlan marks plan as inactive
- [ ] Archived plans not returned in active list
- [ ] Existing subscriptions unaffected by archive

## Integration Tests

- [ ] Create plan with valid data succeeds
- [ ] Create plan with usage components succeeds
- [ ] Get plan returns complete plan
- [ ] List active plans excludes archived
- [ ] Archive plan succeeds
