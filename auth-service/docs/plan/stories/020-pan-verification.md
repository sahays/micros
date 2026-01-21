# Story: PAN Verification

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P2

## Summary

Implement PAN card verification using NSDL/UTIITSL API for identity verification.

## Tasks

- [ ] Create `services/kyc/pan.rs`
- [ ] Implement PAN verification API call
- [ ] Parse and store verified attributes from response
- [ ] Implement `InitiateKyc` RPC for PAN
- [ ] Implement `CompleteKyc` RPC for PAN
- [ ] Add configuration for NSDL credentials
- [ ] Handle API errors and retries

## NSDL Integration

### Verify PAN
**Endpoint:** NSDL PAN Verification API
**Input:** PAN number, Name (optional for matching)
**Output:** PAN status, Name on PAN, Match result

## Attributes from PAN

| Attribute Type | Source Field |
|----------------|--------------|
| pan | Input PAN number |
| name | Name on PAN card |

## Name Matching

PAN verification can include name matching:
- Exact match: 100% confidence
- Fuzzy match: Calculated confidence score
- No match: Verification fails

## Configuration

| Variable | Description |
|----------|-------------|
| NSDL_API_URL | NSDL API endpoint |
| NSDL_CLIENT_ID | API client ID |
| NSDL_CLIENT_SECRET | API secret |

## Acceptance Criteria

- [ ] PAN format validated (AAAAA9999A)
- [ ] PAN status verified (active/inactive)
- [ ] Name from PAN stored as attribute
- [ ] Name matching score calculated if name provided
- [ ] Identity level upgraded to full_kyc
- [ ] Invalid PAN returns clear error
- [ ] Rate limiting per identity
- [ ] Audit event logged with PAN (masked)
