# Story: Adjustments

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement CreateAdjustment and ListAdjustments gRPC methods for resolving discrepancies by creating ledger adjustment entries.

## Tasks

- [ ] Define proto messages: Adjustment, AdjustmentType enum
- [ ] Define proto messages: CreateAdjustmentRequest/Response
- [ ] Define proto messages: ListAdjustmentsRequest/Response
- [ ] Implement CreateAdjustment handler
- [ ] Implement ListAdjustments handler
- [ ] Integrate with ledger-service for journal entry creation
- [ ] Link adjustment to reconciliation
- [ ] Update reconciliation difference after adjustment
- [ ] Add capability checks to all methods

## gRPC Methods

### CreateAdjustment
**Input:** tenant_id (from auth), reconciliation_id, adjustment_type, amount, description
**Output:** adjustment with ledger_entry_id

**Behavior:**
1. Validate reconciliation is in progress (not completed or abandoned)
2. Create journal entry via ledger-service based on adjustment_type
3. Store adjustment record with ledger reference
4. Recalculate reconciliation difference
5. Return adjustment with ledger_entry_id

**Capability:** `reconciliation.adjustment:create`

### ListAdjustments
**Input:** tenant_id (from auth), reconciliation_id, page_size, page_token
**Output:** adjustments[], next_page_token

**Sorting:** Most recent first (by created_utc descending)

**Capability:** `reconciliation.adjustment:read`

## Adjustment Types

### bank_fee
Bank charges not yet recorded in ledger
- Debit: Bank Fees Expense
- Credit: Bank Account (the reconciled account)

### bank_interest
Interest earned not yet recorded in ledger
- Debit: Bank Account
- Credit: Interest Income

### timing_difference
Item cleared in different period than expected
- May need temporary adjustment
- Description should explain the timing issue

### correction
Fix for previous error in ledger
- Debit/Credit based on error type
- Description should reference original entry

### other
Miscellaneous adjustment
- Generic type for edge cases
- Description must explain the adjustment

## Ledger Integration

For each adjustment type, create appropriate journal entry via ledger-service:

```rust
// Example: Bank fee adjustment
let journal_entry = CreateJournalEntryRequest {
    tenant_id: tenant_id.clone(),
    description: format!("Bank fee adjustment: {}", adjustment.description),
    entries: vec![
        JournalLine {
            account_id: bank_fees_expense_account_id,
            debit: Some(amount),
            credit: None,
        },
        JournalLine {
            account_id: bank_account.ledger_account_id,
            debit: None,
            credit: Some(amount),
        },
    ],
    reference: format!("RECON-ADJ-{}", adjustment_id),
};
```

## Common Scenarios

**Bank fees:**
Statement shows fee not in ledger
- Create bank_fee adjustment
- Reduces bank balance in ledger to match statement

**Bank interest:**
Statement shows interest earned not in ledger
- Create bank_interest adjustment
- Increases bank balance in ledger to match statement

**Outstanding checks:**
Check issued but not yet cleared by bank
- Usually timing_difference
- May not need adjustment if it clears next period

**Deposits in transit:**
Deposit made but not yet on statement
- Usually timing_difference
- Track for next reconciliation

**Ledger errors:**
Entry with wrong amount or wrong account
- Create correction adjustment
- Reference the original entry in description

## Business Rules

From spec:
- "Completed reconciliations are immutable; corrections require new entries"
- Cannot add adjustments to completed reconciliation
- Adjustments must bring difference closer to zero (warning if not)

## Acceptance Criteria

- [ ] CreateAdjustment creates ledger journal entry via ledger-service
- [ ] CreateAdjustment links adjustment to reconciliation
- [ ] CreateAdjustment stores ledger_entry_id reference
- [ ] CreateAdjustment recalculates reconciliation difference
- [ ] ListAdjustments returns adjustments for reconciliation
- [ ] ListAdjustments supports pagination
- [ ] Multiple adjustments can be created for one reconciliation
- [ ] Adjustment fails gracefully if ledger-service unavailable
- [ ] Cannot adjust completed reconciliation (FAILED_PRECONDITION)
- [ ] Cannot adjust abandoned reconciliation (FAILED_PRECONDITION)
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Create bank_fee adjustment succeeds
- [ ] Create bank_interest adjustment succeeds
- [ ] Adjustment creates correct ledger journal entry
- [ ] Reconciliation difference updated after adjustment
- [ ] List adjustments returns all adjustments for reconciliation
- [ ] Multiple adjustments accumulate correctly
- [ ] Adjust completed reconciliation returns FAILED_PRECONDITION
- [ ] Adjust abandoned reconciliation returns FAILED_PRECONDITION
- [ ] Ledger-service failure returns appropriate error
- [ ] Operations without capability return PERMISSION_DENIED
