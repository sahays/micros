# Story: Email & Phone Verification

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P1

## Summary

Implement OTP-based verification for email and phone attributes using existing OTP infrastructure.

## Tasks

- [ ] Implement `VerifyAttribute` RPC
- [ ] Integrate with existing OTP send/verify handlers
- [ ] Update attribute verification_status on success
- [ ] Store verification_method and verified_utc
- [ ] Handle verification expiry
- [ ] Add rate limiting for verification attempts

## gRPC Methods

### VerifyAttribute (initiate)
**Input:** identity_id, attribute_id
**Output:** verification_id, otp_sent_to (masked)

**Logic:**
1. Get attribute from identity
2. Send OTP via notification-service (email or SMS based on type)
3. Return verification_id

### VerifyAttribute (complete)
**Input:** verification_id, otp_code
**Output:** success, attribute (with updated status)

**Logic:**
1. Validate OTP
2. Update attribute: verification_status = 'verified', verified_utc = now()
3. Check if identity should upgrade level

## Verification Levels

| Verified Attributes | Identity Level |
|---------------------|----------------|
| None | none |
| Email OR Phone | basic |
| Aadhaar OR PAN | full_kyc |

## Acceptance Criteria

- [ ] Email verification sends OTP to email address
- [ ] Phone verification sends OTP via SMS
- [ ] OTP expires after 10 minutes
- [ ] Max 3 verification attempts per attribute per hour
- [ ] Successful verification updates attribute status
- [ ] Identity level auto-upgrades when threshold met
- [ ] Duplicate verified attribute blocked (unique constraint)
- [ ] Audit event logged on verification
