# Story: Aadhaar KYC

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P1

## Summary

Implement Aadhaar-based KYC verification using UIDAI API for identity verification.

## Tasks

- [ ] Create `services/kyc/aadhaar.rs`
- [ ] Implement UIDAI OTP request API
- [ ] Implement UIDAI OTP verify API
- [ ] Parse and store verified attributes from response
- [ ] Implement `InitiateKyc` RPC for Aadhaar
- [ ] Implement `CompleteKyc` RPC for Aadhaar
- [ ] Add configuration for UIDAI credentials
- [ ] Handle API errors and retries

## UIDAI Integration

### Initiate (OTP Request)
**Endpoint:** UIDAI OTP API
**Input:** Aadhaar number
**Output:** Transaction ID, OTP sent to registered mobile

### Complete (OTP Verify)
**Endpoint:** UIDAI eKYC API
**Input:** Transaction ID, OTP
**Output:** Name, DOB, Gender, Address, Photo (encrypted)

## Attributes from Aadhaar

| Attribute Type | Source Field |
|----------------|--------------|
| aadhaar | Input Aadhaar number |
| name | poi.name |
| dob | poi.dob |
| gender | poi.gender |
| address | poa (concatenated) |

## Configuration

| Variable | Description |
|----------|-------------|
| UIDAI_API_URL | UIDAI API endpoint |
| UIDAI_AUA_CODE | Authorized User Agency code |
| UIDAI_SUB_AUA_CODE | Sub-AUA code |
| UIDAI_LICENSE_KEY | API license key |

## Acceptance Criteria

- [ ] Aadhaar number validated (12 digits, checksum)
- [ ] OTP sent to Aadhaar-registered mobile
- [ ] Successful verification stores all attributes
- [ ] All attributes marked verification_method = 'aadhaar_otp'
- [ ] Identity level upgraded to full_kyc
- [ ] UIDAI API errors handled gracefully
- [ ] Rate limiting per identity (1 attempt per hour)
- [ ] Audit event includes Aadhaar (masked) for compliance
