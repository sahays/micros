# Story: Tax Rates

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement CreateTaxRate and ListTaxRates gRPC methods for managing configurable tax rates.

## Tasks

- [ ] Define proto messages: TaxRate, CreateTaxRateRequest/Response
- [ ] Define proto messages: ListTaxRatesRequest/Response
- [ ] Implement CreateTaxRate handler
- [ ] Implement ListTaxRates handler with filters
- [ ] Implement tax rate effective date logic
- [ ] Implement tax rate deactivation (soft delete)

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

### ListTaxRates
**Input:** tenant_id, is_active (optional), as_of_date (optional), page_size, page_token
**Output:** tax_rates[], next_page_token

**Filtering:**
- is_active filters by active status
- as_of_date filters to rates effective on that date

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

- [ ] CreateTaxRate creates rate with valid data
- [ ] CreateTaxRate rejects invalid rate values
- [ ] CreateTaxRate rejects invalid date ranges
- [ ] ListTaxRates returns tenant's tax rates
- [ ] ListTaxRates filters by active status
- [ ] ListTaxRates filters by effective date
- [ ] Tax rates are immutable once used in invoices
- [ ] Deactivation prevents use in new line items

## Integration Tests

- [ ] Create tax rate with valid data succeeds
- [ ] Create tax rate with rate > 1 returns INVALID_ARGUMENT
- [ ] List active tax rates returns only active
- [ ] List rates as of date returns effective rates
- [ ] Deactivated rate not returned in active list
