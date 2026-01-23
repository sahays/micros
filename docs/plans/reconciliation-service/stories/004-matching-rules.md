# Story: Matching Rules

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement CreateMatchingRule, ListMatchingRules, and DeleteMatchingRule gRPC methods for automated transaction matching based on patterns.

## Tasks

- [ ] Define proto messages: MatchingRule, CreateMatchingRuleRequest/Response
- [ ] Define proto messages: ListMatchingRulesRequest/Response, DeleteMatchingRuleRequest/Response
- [ ] Implement CreateMatchingRule handler
- [ ] Implement ListMatchingRules handler
- [ ] Implement DeleteMatchingRule handler
- [ ] Implement rule matching engine
- [ ] Implement auto-match on statement import

## gRPC Methods

### CreateMatchingRule
**Input:** tenant_id, name, description_pattern, match_type, target_account_id, priority
**Output:** matching_rule

**Validation:**
- name is not empty
- description_pattern is valid for match_type
- target_account_id exists in ledger
- priority is non-negative

### ListMatchingRules
**Input:** tenant_id, is_active (optional), page_size, page_token
**Output:** rules[] (ordered by priority descending)

### DeleteMatchingRule
**Input:** tenant_id, rule_id
**Output:** success

## Match Types

### exact
- Description must match pattern exactly
- Case-insensitive

### contains
- Description must contain pattern
- Case-insensitive

### regex
- Description must match regex pattern
- Full regex support

## Rule Application

Rules are applied in priority order (highest first):
1. For each unmatched bank transaction
2. For each rule (by priority):
   - If description matches pattern
   - Find ledger entries for target_account_id
   - Match by amount and date proximity
   - If match found, create auto match
   - Stop checking rules for this transaction

## Example Rules

| Name | Pattern | Type | Account |
|------|---------|------|---------|
| Stripe Payouts | STRIPE PAYOUT | contains | Bank Account |
| AWS Charges | AWS.Amazon | contains | Cloud Expenses |
| Rent Payment | ^RENT-\d+ | regex | Rent Expense |

## Acceptance Criteria

- [ ] CreateMatchingRule creates rule with valid pattern
- [ ] CreateMatchingRule validates regex patterns
- [ ] ListMatchingRules returns rules by priority
- [ ] DeleteMatchingRule removes rule
- [ ] Rules applied on statement import
- [ ] Higher priority rules checked first
- [ ] Matched transactions marked as auto-matched

## Integration Tests

- [ ] Create rule with contains pattern succeeds
- [ ] Create rule with invalid regex returns INVALID_ARGUMENT
- [ ] Rules auto-match on statement import
- [ ] Priority order respected
- [ ] Delete rule succeeds
