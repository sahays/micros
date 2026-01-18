# Story: OTP Authentication

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P1

## Summary

Implement OTP (One-Time Password) authentication for passwordless login and verification flows via email, SMS, and WhatsApp channels.

## Tasks

- [x] Create `handlers/otp.rs` - OTP send/verify endpoints
- [x] Implement OTP generation with configurable length
- [x] Implement OTP hashing for storage
- [x] Add rate limiting for OTP requests
- [x] Add attempt tracking and lockout
- [x] Implement email channel via existing EmailService
- [x] Add SMS channel placeholder (Twilio integration)
- [x] Add WhatsApp channel placeholder

## Database

Uses existing `otp_codes` table:
```sql
CREATE TABLE otp_codes (
  otp_id UUID PRIMARY KEY,
  tenant_id UUID REFERENCES tenants(tenant_id),
  destination_text TEXT NOT NULL,
  channel_code TEXT NOT NULL CHECK (channel_code IN ('email','sms','whatsapp')),
  purpose_code TEXT NOT NULL CHECK (purpose_code IN ('login','verify_email','verify_phone','reset_password')),
  code_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  consumed_utc TIMESTAMPTZ,
  attempt_count INT NOT NULL DEFAULT 0,
  attempt_max INT NOT NULL DEFAULT 5,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## API Endpoints

```
POST /auth/otp/send
{
  "tenant_id": "uuid",
  "destination": "user@example.com" | "+1234567890",
  "channel": "email" | "sms" | "whatsapp",
  "purpose": "login" | "verify_email" | "verify_phone" | "reset_password"
}
Response: { "otp_id": "uuid", "expires_in": 300 }

POST /auth/otp/verify
{
  "otp_id": "uuid",
  "code": "123456"
}
Response (login): { "access_token": "...", "refresh_token": "...", "expires_in": 900 }
Response (verify): { "verified": true }
```

## OTP Configuration

- Length: 6 digits
- Expiry: 5 minutes
- Max attempts: 5
- Rate limit: 3 OTPs per destination per 15 minutes

## Acceptance Criteria

- [x] OTP generated and hashed before storage
- [x] Email OTP delivery working
- [x] SMS/WhatsApp placeholders ready for integration
- [x] Rate limiting prevents abuse
- [x] Attempt tracking locks out after max failures
- [x] OTP consumed after successful verification
- [x] Login purpose returns tokens
- [x] Verify purpose marks identity as verified

## Implementation Notes

- Phone-based login (SMS/WhatsApp) currently returns error - requires phone->user lookup implementation
- Phone verification OTP verify succeeds but doesn't update user record - needs phone->user lookup
- Email is the primary supported channel for all OTP purposes
