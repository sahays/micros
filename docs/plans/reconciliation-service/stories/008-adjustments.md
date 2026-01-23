# Story: Adjustments

- [ ] **Status: Planning**
- **Epic:** [001-reconciliation-service](../epics/001-reconciliation-service.md)

## Summary

Implement CreateAdjustment gRPC method for resolving discrepancies by creating ledger adjustment entries.

## Tasks

- [ ] Define proto messages: Adjustment, CreateAdjustmentRequest/Response
- [ ] Implement CreateAdjustment handler
- [ ] Integrate with ledger-service for journal entry creation
- [ ] Link adjustment to reconciliation
- [ ] Update reconciliation difference after adjustment

## gRPC Methods

### CreateAdjustment
**Input:** tenant_id, reconciliation_id, adjustment_type, amount, description, debit_account_id, credit_account_id
**Output:** adjustment with journal_id

**Behavior:**
1. Validate reconciliation is in progress
2. Create journal entry via ledger-service
3. Store adjustment record
4. Recalculate reconciliation difference
5. Return adjustment with ledger reference

## Adjustment Types

### bank_fee
Bank charges not yet recorded
- Debit: Bank Fees Expense
- Credit: Bank Account

### bank_interest
Interest earned not yet recorded
- Debit: Bank Account
- Credit: Interest Income

### timing_difference
Item cleared in different period
- Temporary adjustment, may reverse next period

### error_correction
Fix for previous error
- Debit/Credit based on error type

### other
Miscellaneous adjustment with custom accounts

## Common Scenarios

**Bank fees:**
Statement shows fee not in ledger
- Create adjustment: Debit expense, Credit bank

**Outstanding checks:**
Check issued but not yet cleared
- May not need adjustment (timing only)
- Track as reconciling item

**Deposits in transit:**
Deposit made but not yet on statement
- May not need adjustment (timing only)
- Track as reconciling item

**Errors:**
Ledger entry with wrong amount
- Create correcting entry

## Acceptance Criteria

- [ ] CreateAdjustment creates ledger journal entry
- [ ] CreateAdjustment links to reconciliation
- [ ] CreateAdjustment recalculates difference
- [ ] Adjustment reduces reconciliation difference
- [ ] Multiple adjustments can be created
- [ ] Adjustment reverts if ledger call fails
- [ ] Cannot adjust completed reconciliation

## Integration Tests

- [ ] Create bank fee adjustment succeeds
- [ ] Adjustment creates correct ledger entry
- [ ] Reconciliation difference updated
- [ ] Multiple adjustments accumulate
- [ ] Adjust completed reconciliation returns FAILED_PRECONDITION
