# Story: Tax Rates

- [x] **Status: Complete**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement CreateTaxRate, GetTaxRate, ListTaxRates, and UpdateTaxRate gRPC methods for managing configurable tax rates.

## Tasks

- [x] Define proto messages: TaxRate, CreateTaxRateRequest/Response
- [x] Define proto messages: GetTaxRateRequest/Response
- [x] Define proto messages: ListTaxRatesRequest/Response
- [x] Define proto messages: UpdateTaxRateRequest/Response
- [x] Implement CreateTaxRate handler
- [x] Implement GetTaxRate handler
- [x] Implement ListTaxRates handler with filters
- [x] Implement UpdateTaxRate handler
- [x] Implement tax rate effective date logic
- [x] Implement tax rate deactivation (soft delete)

## gRPC Methods

### CreateTaxRate
**Input:** tenant_id, name, rate, tax_type, effective_from, effective_to (optional)
**Output:** tax_rate

**Validation:**
- name is not empty
- rate is between 0 and 1 (0% to 100%)
- tax_type is inclusive or exclusive
- effective_from is valid date
- effective_to > effective_from if provided

**Status:** ✅ Implemented

### GetTaxRate
**Input:** tenant_id, tax_rate_id
**Output:** tax_rate

**Status:** ✅ Implemented

### ListTaxRates
**Input:** tenant_id, is_active (optional), as_of_date (optional), page_size, page_token
**Output:** tax_rates[], next_page_token

**Filtering:**
- is_active filters by active status
- as_of_date filters to rates effective on that date

**Status:** ✅ Implemented

### UpdateTaxRate
**Input:** tenant_id, tax_rate_id, name, rate, calculation, effective_from, effective_to, active
**Output:** tax_rate

**Status:** ✅ Implemented

## Tax Types

**Exclusive:** Tax added on top of line item amount
- Line amount: $100
- Tax rate: 18%
- Tax amount: $18
- Total: $118

**Inclusive:** Tax included in line item amount
- Line amount: $118 (includes tax)
- Tax rate: 18%
- Tax amount: $18 (calculated as 118 - 118/1.18)
- Pre-tax amount: $100

## Acceptance Criteria

- [x] CreateTaxRate creates rate with valid data
- [x] CreateTaxRate rejects invalid rate values
- [x] CreateTaxRate rejects invalid date ranges
- [x] GetTaxRate returns tax rate by ID
- [x] GetTaxRate returns NOT_FOUND for missing rate
- [x] ListTaxRates returns tenant's tax rates
- [x] ListTaxRates filters by active status
- [x] ListTaxRates filters by effective date
- [x] UpdateTaxRate modifies existing rate
- [x] Deactivation prevents use in new line items

## Integration Tests

- [x] Create tax rate with valid data succeeds
- [x] Create tax rate with end date succeeds
- [x] Get tax rate returns created rate
- [x] Get nonexistent rate returns NOT_FOUND
- [x] List tax rates returns tenant's rates
- [x] List active tax rates filters inactive
- [x] Update tax rate succeeds
- [x] Deactivated rate not returned in active list
