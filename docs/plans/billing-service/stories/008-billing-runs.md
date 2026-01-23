# Story: Billing Runs

- [ ] **Status: Planning**
- **Epic:** [001-billing-service](../epics/001-billing-service.md)

## Summary

Implement RunBilling, RunBillingForSubscription, GetBillingRun, and ListBillingRuns gRPC methods for executing billing operations and generating invoices.

## Tasks

- [ ] Define proto messages: BillingRun, BillingRunStatus enum, BillingRunType enum
- [ ] Define proto messages: BillingRunResult
- [ ] Define proto messages: RunBillingRequest/Response
- [ ] Define proto messages: RunBillingForSubscriptionRequest/Response
- [ ] Define proto messages: GetBillingRunRequest/Response
- [ ] Define proto messages: ListBillingRunsRequest/Response
- [ ] Implement RunBilling handler (batch processing)
- [ ] Implement RunBillingForSubscription handler (single subscription)
- [ ] Implement charge calculation logic
- [ ] Implement usage aggregation and charge creation
- [ ] Integrate with invoicing-service for invoice creation
- [ ] Implement GetBillingRun handler
- [ ] Implement ListBillingRuns handler
- [ ] Add capability checks to all methods
- [ ] Add metering for billing operations

## gRPC Methods

### RunBilling
**Input:** tenant_id, run_type (scheduled, manual)
**Output:** billing_run

**Validation:**
- Caller must have execute capability

**Business Logic:**
1. Create billing_run record with status "running"
2. Find all subscriptions due for billing (current_period_end <= today)
3. For each subscription:
   - Calculate recurring charge (base_price from plan)
   - Aggregate usage and create usage charges
   - Create invoice via invoicing-service
   - Record result (success/failed)
4. Update billing_run with completion status and counts

**Capability:** `billing.run:execute`

### RunBillingForSubscription
**Input:** tenant_id, subscription_id
**Output:** billing_run_result

**Validation:**
- Subscription must exist and be active
- Subscription must have billable cycle

**Business Logic:**
1. Create billing_run with type "single"
2. Process single subscription
3. Return result immediately

**Capability:** `billing.run:execute`

### GetBillingRun
**Input:** tenant_id, run_id
**Output:** billing_run with results[]

**Capability:** `billing.run:read`

### ListBillingRuns
**Input:** tenant_id, status (optional), run_type (optional), page_size, page_token
**Output:** billing_runs[], next_page_token

**Capability:** `billing.run:read`

## Billing Run Types

| Type | Description |
|------|-------------|
| `SCHEDULED` | Automatic daily/hourly run |
| `MANUAL` | Admin-triggered batch run |
| `SINGLE` | Single subscription billing |

## Billing Run Status

| Status | Description |
|--------|-------------|
| `RUNNING` | Currently processing subscriptions |
| `COMPLETED` | All subscriptions processed |
| `FAILED` | Run failed (partial or complete) |

## Charge Calculation

For each subscription billing cycle:

1. **Recurring Charge:**
   ```
   charge_type: "recurring"
   description: "Monthly subscription - Pro Plan"
   quantity: 1
   unit_price: plan.base_price
   amount: plan.base_price
   ```

2. **Usage Charges (per component):**
   ```
   total_usage = SUM(usage_records.quantity) for cycle
   billable_units = MAX(0, total_usage - component.included_units)
   
   charge_type: "usage"
   description: "API Calls (1,500 over 1,000 included)"
   quantity: billable_units
   unit_price: component.unit_price
   amount: billable_units * unit_price
   ```

3. **Proration Charges:** (created by ChangePlan, not billing run)

## Invoicing Integration

Call invoicing-service to create invoice:
```rust
let invoice_request = CreateInvoiceRequest {
    tenant_id,
    customer_id: subscription.customer_id,
    currency: plan.currency,
    line_items: charges.map(|c| InvoiceLineItem {
        description: c.description,
        quantity: c.quantity,
        unit_price: c.unit_price,
        amount: c.amount,
    }),
    due_date: calculate_due_date(30), // 30 days from issue
};

let invoice = invoicing_client.create_invoice(invoice_request).await?;
cycle.invoice_id = invoice.invoice_id;
cycle.status = "invoiced";
```

## Metering

Record on each operation:
```rust
record_billing_run(&tenant_id, &run_type, &status);
record_billing_run_subscriptions(&tenant_id, processed, succeeded, failed);
record_invoice_created(&tenant_id);
```

## Acceptance Criteria

- [ ] RunBilling processes due subscriptions
- [ ] RunBilling creates recurring charges
- [ ] RunBilling creates usage charges
- [ ] RunBilling creates invoices via invoicing-service
- [ ] RunBilling records success/failure per subscription
- [ ] RunBillingForSubscription processes single subscription
- [ ] RunBillingForSubscription returns immediate result
- [ ] GetBillingRun returns run with results
- [ ] GetBillingRun returns NOT_FOUND for missing run
- [ ] ListBillingRuns filters by status and type
- [ ] ListBillingRuns pagination works correctly
- [ ] Paused subscriptions are skipped
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Run billing processes subscriptions due for billing
- [ ] Run billing creates recurring charges correctly
- [ ] Run billing aggregates usage and creates charges
- [ ] Run billing creates invoices via invoicing-service
- [ ] Run billing skips paused subscriptions
- [ ] Run billing skips cancelled subscriptions
- [ ] Run billing for subscription processes single sub
- [ ] Run billing for subscription returns result
- [ ] Get billing run returns complete run with results
- [ ] List billing runs filters by status
- [ ] List billing runs filters by type
- [ ] List billing runs pagination works
- [ ] Billing run with invoicing failure records error
- [ ] Operations without capability return PERMISSION_DENIED
