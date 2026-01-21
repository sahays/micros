# Story: Attribute Normalization

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P0

## Summary

Implement consistent normalization for all attribute types to enable exact and fuzzy matching.

## Tasks

- [ ] Create `services/identity/normalizer.rs`
- [ ] Implement email normalization
- [ ] Implement phone normalization (E.164)
- [ ] Implement Aadhaar normalization
- [ ] Implement PAN normalization
- [ ] Implement name normalization
- [ ] Add validation for each attribute type
- [ ] Unit tests for all normalizers

## Normalization Rules

| Type | Input Example | Normalized Output |
|------|---------------|-------------------|
| email | Alice@Example.COM | alice@example.com |
| phone | +91 98765 43210 | +919876543210 |
| phone | 09876543210 | +919876543210 |
| aadhaar | 1234 5678 9012 | 123456789012 |
| aadhaar | 1234-5678-9012 | 123456789012 |
| pan | abcde1234f | ABCDE1234F |
| name | Dr. Alice Smith Jr. | alice smith |
| name | ALICE  SMITH | alice smith |
| dob | 15/01/1990 | 1990-01-15 |
| dob | Jan 15, 1990 | 1990-01-15 |

## Validation Rules

| Type | Validation |
|------|------------|
| email | RFC 5322 compliant |
| phone | Valid E.164 after normalization |
| aadhaar | Exactly 12 digits, valid checksum |
| pan | Format: AAAAA9999A |
| name | Non-empty after normalization |
| dob | Valid date, not in future |

## Acceptance Criteria

- [ ] All attribute types have normalizer function
- [ ] Normalization is idempotent (normalize(normalize(x)) == normalize(x))
- [ ] Invalid inputs return validation error
- [ ] Aadhaar checksum validation implemented
- [ ] PAN format validation implemented
- [ ] Phone numbers default to +91 country code
- [ ] Names stripped of titles, suffixes, extra whitespace
- [ ] Unit tests cover edge cases
