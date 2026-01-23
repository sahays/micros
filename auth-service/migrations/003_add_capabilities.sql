-- Add missing capabilities to align with auth-service spec

INSERT INTO capabilities (cap_id, cap_key) VALUES
  -- Capability registry
  (uuid_generate_v4(), 'capability:read'),
  -- Invitation management
  (uuid_generate_v4(), 'invitation:read'),
  (uuid_generate_v4(), 'invitation:revoke'),
  -- Visibility read
  (uuid_generate_v4(), 'visibility:read'),
  -- Tenant management
  (uuid_generate_v4(), 'tenant:read'),
  (uuid_generate_v4(), 'tenant:update')
ON CONFLICT (cap_key) DO NOTHING;
