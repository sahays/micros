# Story: Billing Runs

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement RunBilling, GetBillingRun, and ListBillingRuns gRPC methods for executing billing cycles and generating invoices.

## Tasks

- [ ] Define proto messages: BillingRun, RunBillingRequest/Response
- [ ] Define proto messages: GetBillingRunRequest/Response, ListBillingRunsRequest/Response
- [ ] Implement RunBilling handler (batch processing)
- [ ] Implement single subscription billing logic
- [ ] Implement charge calculation (base + usage)
- [ ] Integrate with invoicing-service for invoice creation
- [ ] Implement billing cycle advancement
- [ ] Handle partial failures with retry

## gRPC Methods

### RunBilling
**Input:** tenant_id, run_date (optional, defaults to today), subscription_id (optional for single)
**Output:** billing_run

**Behavior:**
1. Create billing_run record
2. Find subscriptions due for billing (current_period_end <= run_date)
3. For each subscription:
   - Calculate charges (base price + usage overage)
   - Create invoice via invoicing-service
   - Create billing_cycle record
   - Advance subscription to next period
4. Update billing_run totals
5. Return billing_run summary

### GetBillingRun
**Input:** tenant_id, run_id
**Output:** billing_run with cycle details

### ListBillingRuns
**Input:** tenant_id, date_from (optional), date_to (optional), status (optional), page_size, page_token
**Output:** billing_runs[], next_page_token

## Charge Calculation

For each subscription:
1. **Base charge:** plan.base_price
2. **Usage charges:** For each component:
   - total_usage = sum of usage in period
   - overage = max(0, total_usage - included_units)
   - charge = overage × unit_price
3. **Total:** base_charge + sum(usage_charges)

## Invoice Creation

Call invoicing-service with:
- customer_id from subscription
- Line item for base plan charge
- Line items for each usage component with overage
- Due date based on payment terms

## Failure Handling

- Individual subscription failures don't stop the run
- Failed subscriptions marked in billing_cycle
- Retry logic with exponential backoff
- Alert on repeated failures

## Acceptance Criteria

- [ ] RunBilling processes all due subscriptions
- [ ] RunBilling calculates correct charges
- [ ] RunBilling creates invoices via invoicing-service
- [ ] RunBilling advances billing periods
- [ ] RunBilling handles partial failures
- [ ] GetBillingRun returns run details
- [ ] ListBillingRuns filters correctly
- [ ] Paused subscriptions skipped

## Integration Tests

- [ ] Billing run processes due subscriptions
- [ ] Billing run skips not-yet-due subscriptions
- [ ] Billing run skips paused subscriptions
- [ ] Invoice created with correct line items
- [ ] Subscription period advanced after billing
- [ ] Failed billing logged but run continues
