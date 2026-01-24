# Story: Matching Rules

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement CreateMatchingRule, GetMatchingRule, ListMatchingRules, UpdateMatchingRule, and DeleteMatchingRule gRPC methods for automated transaction matching based on patterns.

## Tasks

- [ ] Define proto messages: MatchingRule, CreateMatchingRuleRequest/Response
- [ ] Define proto messages: GetMatchingRuleRequest/Response
- [ ] Define proto messages: ListMatchingRulesRequest/Response
- [ ] Define proto messages: UpdateMatchingRuleRequest/Response
- [ ] Define proto messages: DeleteMatchingRuleRequest/Response
- [ ] Implement CreateMatchingRule handler
- [ ] Implement GetMatchingRule handler
- [ ] Implement ListMatchingRules handler
- [ ] Implement UpdateMatchingRule handler
- [ ] Implement DeleteMatchingRule handler
- [ ] Implement rule matching engine
- [ ] Implement auto-match on statement commit
- [ ] Add capability checks to all methods

## gRPC Methods

### CreateMatchingRule
**Input:** tenant_id (from auth), name, description_pattern, match_type, target_account_id (optional), priority (optional)
**Output:** matching_rule

**Validation:**
- name is not empty
- description_pattern is valid for match_type
- If provided, target_account_id exists in ledger
- priority defaults to 0 if not provided

**Capability:** `reconciliation.matching_rule:create`

### GetMatchingRule
**Input:** tenant_id (from auth), rule_id
**Output:** matching_rule

**Capability:** `reconciliation.matching_rule:read`

### ListMatchingRules
**Input:** tenant_id (from auth), active_only (optional), page_size, page_token
**Output:** rules[] (ordered by priority ascending - lower number = higher priority)

**Capability:** `reconciliation.matching_rule:read`

### UpdateMatchingRule
**Input:** tenant_id (from auth), rule_id, name (optional), description_pattern (optional), match_type (optional), target_account_id (optional), priority (optional), is_active (optional)
**Output:** matching_rule

**Validation:**
- rule_id exists and belongs to tenant
- If updating pattern, validate for match_type
- If updating target_account_id, validate exists in ledger

**Capability:** `reconciliation.matching_rule:update`

### DeleteMatchingRule
**Input:** tenant_id (from auth), rule_id
**Output:** success

**Note:** Soft delete by setting is_active = false is preferred. Hard delete only if rule was never used.

**Capability:** `reconciliation.matching_rule:delete`

## Match Types

### exact
- Description must match pattern exactly
- Case-insensitive

### contains
- Description must contain pattern
- Case-insensitive

### starts_with
- Description must start with pattern
- Case-insensitive

### ends_with
- Description must end with pattern
- Case-insensitive

### regex
- Description must match regex pattern
- Full regex support

## Rule Application

Per spec: "Matching rules are evaluated in priority order; first match wins"

Rules are applied in priority order (lowest number first):
1. For each unmatched bank transaction
2. For each active rule (by priority ascending):
   - If description matches pattern
   - Find ledger entries for target_account_id (if specified)
   - Match by amount and date proximity
   - If match found, create auto match
   - Stop checking rules for this transaction (first match wins)

## Example Rules

| Name | Pattern | Type | Priority | Account |
|------|---------|------|----------|---------|
| Stripe Payouts | STRIPE PAYOUT | contains | 1 | Bank Account |
| AWS Charges | AWS.Amazon | contains | 2 | Cloud Expenses |
| Rent Payment | ^RENT-\d+ | regex | 3 | Rent Expense |
| Salary Credit | SALARY | contains | 10 | Payroll |

## Acceptance Criteria

- [ ] CreateMatchingRule creates rule with valid pattern
- [ ] CreateMatchingRule validates regex patterns
- [ ] ListMatchingRules returns rules ordered by priority
- [ ] ListMatchingRules supports active_only filter
- [ ] UpdateMatchingRule updates specified fields
- [ ] UpdateMatchingRule validates new pattern/account
- [ ] DeleteMatchingRule removes rule (or deactivates)
- [ ] Rules applied on statement commit
- [ ] Lower priority number checked first
- [ ] First matching rule wins (no subsequent rules checked)
- [ ] Matched transactions marked as auto-matched
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create rule with contains pattern succeeds
- [ ] Create rule with invalid regex returns INVALID_ARGUMENT
- [ ] Update rule changes pattern correctly
- [ ] Update inactive rule to active works
- [ ] Rules auto-match on statement commit
- [ ] Priority order respected (lower number first)
- [ ] First match wins - no double matching
- [ ] Delete rule succeeds
- [ ] List with active_only=true filters correctly
- [ ] Operations without capability return PERMISSION_DENIED
