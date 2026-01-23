-- Add the superadmin wildcard capability
-- This capability grants access to all endpoints when assigned to a role

INSERT INTO capabilities (cap_id, cap_key, created_utc)
VALUES (gen_random_uuid(), '*', NOW())
ON CONFLICT (cap_key) DO NOTHING;
