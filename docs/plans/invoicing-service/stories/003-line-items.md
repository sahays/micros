# Story: Line Items

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement AddLineItem, UpdateLineItem, and RemoveLineItem gRPC methods for managing invoice line items.

## Tasks

- [ ] Define proto messages: LineItem, AddLineItemRequest/Response
- [ ] Define proto messages: UpdateLineItemRequest/Response, RemoveLineItemRequest/Response
- [ ] Implement AddLineItem handler with tax calculation
- [ ] Implement UpdateLineItem handler
- [ ] Implement RemoveLineItem handler
- [ ] Implement automatic invoice totals recalculation
- [ ] Implement line number ordering

## gRPC Methods

### AddLineItem
**Input:** tenant_id, invoice_id, description, quantity, unit_price, tax_rate_id (optional), account_id (optional), metadata
**Output:** line_item, updated invoice totals

**Validation:**
- Invoice must be in draft status
- quantity > 0
- unit_price >= 0
- tax_rate_id must exist and be active if provided

### UpdateLineItem
**Input:** tenant_id, invoice_id, line_item_id, description, quantity, unit_price, tax_rate_id, account_id, metadata
**Output:** line_item, updated invoice totals

**Validation:**
- Invoice must be in draft status

### RemoveLineItem
**Input:** tenant_id, invoice_id, line_item_id
**Output:** updated invoice totals

**Validation:**
- Invoice must be in draft status

## Tax Calculation

**Exclusive tax:** tax_amount = amount * tax_rate
**Inclusive tax:** tax_amount = amount - (amount / (1 + tax_rate))

Invoice totals update:
- subtotal = sum of line item amounts
- tax_total = sum of line item tax_amounts
- total = subtotal + tax_total
- balance_due = total - amount_paid

## Acceptance Criteria

- [ ] AddLineItem creates line item with calculated amount
- [ ] AddLineItem applies tax rate correctly
- [ ] AddLineItem updates invoice totals
- [ ] AddLineItem rejects if invoice is not draft
- [ ] UpdateLineItem modifies existing line item
- [ ] UpdateLineItem recalculates invoice totals
- [ ] RemoveLineItem deletes line item
- [ ] RemoveLineItem recalculates invoice totals
- [ ] Line numbers are sequential per invoice

## Integration Tests

- [ ] Add line item to draft invoice succeeds
- [ ] Add line item to issued invoice returns FAILED_PRECONDITION
- [ ] Tax calculation correct for exclusive rate
- [ ] Tax calculation correct for inclusive rate
- [ ] Update line item recalculates totals
- [ ] Remove line item recalculates totals
