# Story: Line Items

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement AddLineItem, UpdateLineItem, and RemoveLineItem gRPC methods for managing invoice line items.

## Tasks

- [x] Define proto messages: LineItem, AddLineItemRequest/Response
- [x] Define proto messages: UpdateLineItemRequest/Response, RemoveLineItemRequest/Response
- [x] Implement AddLineItem handler with tax calculation
- [x] Implement UpdateLineItem handler
- [x] Implement RemoveLineItem handler
- [x] Implement automatic invoice totals recalculation
- [x] Implement line number ordering

## gRPC Methods

### AddLineItem
**Input:** tenant_id, invoice_id, description, quantity, unit_price, tax_rate_id (optional), account_id (optional), metadata
**Output:** line_item, updated invoice totals

**Validation:**
- Invoice must be in draft status
- quantity > 0
- unit_price >= 0
- tax_rate_id must exist and be active if provided

**Status:** ✅ Implemented

### UpdateLineItem
**Input:** tenant_id, invoice_id, line_item_id, description, quantity, unit_price, tax_rate_id, account_id, metadata
**Output:** line_item, updated invoice totals

**Validation:**
- Invoice must be in draft status

**Status:** ✅ Implemented

### RemoveLineItem
**Input:** tenant_id, invoice_id, line_item_id
**Output:** updated invoice totals

**Validation:**
- Invoice must be in draft status

**Status:** ✅ Implemented

## Tax Calculation

**Exclusive tax:** tax_amount = amount * tax_rate
**Inclusive tax:** tax_amount = amount - (amount / (1 + tax_rate))

Invoice totals update:
- subtotal = sum of line item amounts
- tax_total = sum of line item tax_amounts
- total = subtotal + tax_total
- balance_due = total - amount_paid

## Acceptance Criteria

- [x] AddLineItem creates line item with calculated amount
- [x] AddLineItem applies tax rate correctly
- [x] AddLineItem updates invoice totals
- [x] AddLineItem rejects if invoice is not draft
- [x] UpdateLineItem modifies existing line item
- [x] UpdateLineItem recalculates invoice totals
- [x] RemoveLineItem deletes line item
- [x] RemoveLineItem recalculates invoice totals
- [x] Line numbers are sequential per invoice

## Integration Tests

- [x] Add line item to draft invoice succeeds
- [x] Add line item to issued invoice returns FAILED_PRECONDITION
- [x] Tax calculation correct for exclusive rate
- [x] Tax calculation correct for inclusive rate
- [x] Update line item recalculates totals
- [x] Remove line item recalculates totals
- [x] Fractional quantity supported (2.5 hours consulting)
