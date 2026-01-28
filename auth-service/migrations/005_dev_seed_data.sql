-- Development Seed Data
-- Creates a well-known test tenant with admin user for development/testing
-- All IDs are deterministic UUIDs for predictable test data
--
-- Test Tenant ID:  00000000-0000-0000-0000-000000000001
-- Test User ID:    00000000-0000-0000-0000-000000000002
-- Test Org ID:     00000000-0000-0000-0000-000000000003
-- Test Role ID:    00000000-0000-0000-0000-000000000004
--
-- Test User Credentials:
--   Email: admin@test-school.local
--   Password: TestPassword123!
--
-- This migration is idempotent (safe to run multiple times)

-- 1. Create test tenant
INSERT INTO tenants (tenant_id, tenant_slug, tenant_label, tenant_state_code, created_utc)
VALUES (
  '00000000-0000-0000-0000-000000000001',
  'test-school',
  'Test School',
  'active',
  NOW()
)
ON CONFLICT (tenant_id) DO NOTHING;

-- 2. Create test admin user
INSERT INTO users (user_id, tenant_id, email, email_verified, display_name, user_state_code, created_utc)
VALUES (
  '00000000-0000-0000-0000-000000000002',
  '00000000-0000-0000-0000-000000000001',
  'admin@test-school.local',
  TRUE,
  'Test Admin',
  'active',
  NOW()
)
ON CONFLICT (tenant_id, email) DO NOTHING;

-- 3. Create password identity for test user
-- Password: TestPassword123! (argon2id hash)
INSERT INTO user_identities (ident_id, user_id, ident_provider_code, ident_hash, created_utc)
VALUES (
  '00000000-0000-0000-0000-000000000005',
  '00000000-0000-0000-0000-000000000002',
  'password',
  '$argon2id$v=19$m=19456,t=2,p=1$V6FmzIOrn8cKMPy1doLbVw$Vd0+sv5ccjmyu/Qi5JCJyUUqLyTeE9qmPwt14T/qEuM',
  NOW()
)
ON CONFLICT (user_id, ident_provider_code) DO NOTHING;

-- 4. Create root org node for test tenant
INSERT INTO org_nodes (org_node_id, tenant_id, node_type_code, node_label, parent_org_node_id, active_flag, created_utc)
VALUES (
  '00000000-0000-0000-0000-000000000003',
  '00000000-0000-0000-0000-000000000001',
  'root',
  'Test School Root',
  NULL,
  TRUE,
  NOW()
)
ON CONFLICT (org_node_id) DO NOTHING;

-- 5. Create org node path entry (self-reference for root)
INSERT INTO org_node_paths (tenant_id, ancestor_org_node_id, descendant_org_node_id, depth_val)
VALUES (
  '00000000-0000-0000-0000-000000000001',
  '00000000-0000-0000-0000-000000000003',
  '00000000-0000-0000-0000-000000000003',
  0
)
ON CONFLICT (ancestor_org_node_id, descendant_org_node_id) DO NOTHING;

-- 6. Create admin role for test tenant
INSERT INTO roles (role_id, tenant_id, role_label, created_utc)
VALUES (
  '00000000-0000-0000-0000-000000000004',
  '00000000-0000-0000-0000-000000000001',
  'Admin',
  NOW()
)
ON CONFLICT (tenant_id, role_label) DO NOTHING;

-- 7. Assign superadmin (*) capability to admin role
INSERT INTO role_capabilities (role_id, cap_id)
SELECT
  '00000000-0000-0000-0000-000000000004',
  cap_id
FROM capabilities
WHERE cap_key = '*'
ON CONFLICT (role_id, cap_id) DO NOTHING;

-- 8. Assign test user to root org with admin role
INSERT INTO org_assignments (assignment_id, tenant_id, user_id, org_node_id, role_id, start_utc, end_utc)
VALUES (
  '00000000-0000-0000-0000-000000000006',
  '00000000-0000-0000-0000-000000000001',
  '00000000-0000-0000-0000-000000000002',
  '00000000-0000-0000-0000-000000000003',
  '00000000-0000-0000-0000-000000000004',
  NOW(),
  NULL
)
ON CONFLICT (assignment_id) DO NOTHING;
