-- Recreate otp_codes table to match the application model.
-- The original schema (001) had a user_id-based design; the application
-- now uses a destination-based design that supports sending OTPs before
-- a user record exists (e.g. during registration email verification).

DROP INDEX IF EXISTS idx_otp_user;
DROP TABLE IF EXISTS otp_codes;

CREATE TABLE otp_codes (
  otp_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID REFERENCES tenants(tenant_id),
  destination_text TEXT NOT NULL,
  channel_code TEXT NOT NULL,
  purpose_code TEXT NOT NULL,
  code_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  consumed_utc TIMESTAMPTZ,
  attempt_count INT NOT NULL DEFAULT 0,
  attempt_max INT NOT NULL DEFAULT 5,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_otp_destination ON otp_codes(destination_text);
CREATE INDEX idx_otp_tenant ON otp_codes(tenant_id);
