-- Add role:delete capability for DeleteRole RPC
INSERT INTO capabilities (cap_id, cap_key) VALUES
  (uuid_generate_v4(), 'role:delete')
ON CONFLICT (cap_key) DO NOTHING;
