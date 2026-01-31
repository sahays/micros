-- Add phone-based invitation support.
-- Allows invitations to use phone verification instead of email,
-- and carry arbitrary metadata for the inviting context.

ALTER TABLE invitations
  ADD COLUMN phone TEXT,
  ADD COLUMN verification_type TEXT NOT NULL DEFAULT 'email'
    CHECK (verification_type IN ('email', 'phone')),
  ADD COLUMN metadata_json JSONB;

CREATE INDEX idx_invitations_phone ON invitations(phone) WHERE phone IS NOT NULL;
