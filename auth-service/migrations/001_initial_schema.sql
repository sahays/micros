-- Auth Service v2 Initial Schema
-- Capability-based authorization with tenant hierarchy

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Tenants
CREATE TABLE tenants (
  tenant_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_slug TEXT NOT NULL UNIQUE,
  tenant_label TEXT NOT NULL,
  tenant_state_code TEXT NOT NULL CHECK (tenant_state_code IN ('active','suspended')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_tenants_slug ON tenants(tenant_slug);

-- Users
CREATE TABLE users (
  user_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  email TEXT NOT NULL,
  email_verified BOOLEAN NOT NULL DEFAULT FALSE,
  google_id TEXT,
  display_name TEXT,
  user_state_code TEXT NOT NULL CHECK (user_state_code IN ('active','suspended','deactivated')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(tenant_id, email)
);

CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_email ON users(tenant_id, email);

-- User Identities (auth providers)
CREATE TABLE user_identities (
  ident_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  ident_provider_code TEXT NOT NULL CHECK (ident_provider_code IN ('password','google')),
  ident_hash TEXT NOT NULL,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(user_id, ident_provider_code)
);

CREATE INDEX idx_user_identities_user ON user_identities(user_id);

-- Refresh Sessions
CREATE TABLE refresh_sessions (
  session_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  token_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  revoked_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_refresh_user ON refresh_sessions(user_id);
CREATE INDEX idx_refresh_token_hash ON refresh_sessions(token_hash_text);

-- OTP Codes
CREATE TABLE otp_codes (
  otp_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  purpose_code TEXT NOT NULL CHECK (purpose_code IN ('email_verification','password_reset','two_factor_auth')),
  otp_hash TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  used_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_otp_user ON otp_codes(user_id);

-- Org Nodes (hierarchy)
CREATE TABLE org_nodes (
  org_node_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  node_type_code TEXT NOT NULL,
  node_label TEXT NOT NULL,
  parent_org_node_id UUID REFERENCES org_nodes(org_node_id),
  active_flag BOOLEAN NOT NULL DEFAULT TRUE,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_org_nodes_tenant ON org_nodes(tenant_id);
CREATE INDEX idx_org_nodes_parent ON org_nodes(parent_org_node_id);

-- Org Node Paths (closure table for hierarchy queries)
CREATE TABLE org_node_paths (
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  ancestor_org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id) ON DELETE CASCADE,
  descendant_org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id) ON DELETE CASCADE,
  depth_val INT NOT NULL,
  PRIMARY KEY (ancestor_org_node_id, descendant_org_node_id)
);

CREATE INDEX idx_org_path_ancestor ON org_node_paths(ancestor_org_node_id);
CREATE INDEX idx_org_path_descendant ON org_node_paths(descendant_org_node_id);
CREATE INDEX idx_org_path_tenant ON org_node_paths(tenant_id);

-- Roles (tenant-scoped)
CREATE TABLE roles (
  role_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  role_label TEXT NOT NULL,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(tenant_id, role_label)
);

CREATE INDEX idx_roles_tenant ON roles(tenant_id);

-- Capabilities (global registry)
CREATE TABLE capabilities (
  cap_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  cap_key TEXT NOT NULL UNIQUE,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Role Capabilities (mapping)
CREATE TABLE role_capabilities (
  role_id UUID NOT NULL REFERENCES roles(role_id) ON DELETE CASCADE,
  cap_id UUID NOT NULL REFERENCES capabilities(cap_id) ON DELETE CASCADE,
  PRIMARY KEY (role_id, cap_id)
);

-- Org Assignments (time-bounded user→org→role)
CREATE TABLE org_assignments (
  assignment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  role_id UUID NOT NULL REFERENCES roles(role_id),
  start_utc TIMESTAMPTZ NOT NULL,
  end_utc TIMESTAMPTZ,
  CHECK (end_utc IS NULL OR end_utc > start_utc)
);

CREATE INDEX idx_assign_user ON org_assignments(user_id);
CREATE INDEX idx_assign_node ON org_assignments(org_node_id);
CREATE INDEX idx_assign_tenant ON org_assignments(tenant_id);
CREATE INDEX idx_assign_active ON org_assignments(user_id, start_utc, end_utc);

-- Visibility Grants (cross-org visibility)
CREATE TABLE visibility_grants (
  grant_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_vis_user ON visibility_grants(user_id);
CREATE INDEX idx_vis_node ON visibility_grants(org_node_id);

-- Invitations
CREATE TABLE invitations (
  invitation_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  email TEXT NOT NULL,
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  role_id UUID NOT NULL REFERENCES roles(role_id),
  token_hash TEXT NOT NULL,
  state_code TEXT NOT NULL CHECK (state_code IN ('pending','accepted','expired','revoked')),
  expiry_utc TIMESTAMPTZ NOT NULL,
  accepted_utc TIMESTAMPTZ,
  created_by_user_id UUID NOT NULL REFERENCES users(user_id),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invitations_tenant ON invitations(tenant_id);
CREATE INDEX idx_invitations_token ON invitations(token_hash);

-- Audit Events (immutable log)
CREATE TABLE audit_events (
  event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID REFERENCES tenants(tenant_id),
  actor_user_id UUID REFERENCES users(user_id),
  actor_svc_id UUID,
  event_type_code TEXT NOT NULL,
  target_type TEXT,
  target_id UUID,
  event_data JSONB,
  ip_address TEXT,
  user_agent TEXT,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_tenant_time ON audit_events(tenant_id, created_utc DESC);
CREATE INDEX idx_audit_entity ON audit_events(target_type, target_id);

-- Services (Know-Your-Service registry)
CREATE TABLE services (
  svc_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID REFERENCES tenants(tenant_id),
  svc_key TEXT NOT NULL UNIQUE,
  svc_label TEXT NOT NULL,
  svc_state_code TEXT NOT NULL CHECK (svc_state_code IN ('active','disabled')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Service Secrets
CREATE TABLE service_secrets (
  secret_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  svc_id UUID NOT NULL REFERENCES services(svc_id) ON DELETE CASCADE,
  secret_hash_text TEXT NOT NULL,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_utc TIMESTAMPTZ
);

CREATE INDEX idx_svc_secret_svc ON service_secrets(svc_id);

-- Service Permissions
CREATE TABLE service_permissions (
  svc_id UUID NOT NULL REFERENCES services(svc_id) ON DELETE CASCADE,
  perm_key TEXT NOT NULL,
  PRIMARY KEY (svc_id, perm_key)
);

-- Service Sessions (optional token-based auth)
CREATE TABLE service_sessions (
  svc_session_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  svc_id UUID NOT NULL REFERENCES services(svc_id) ON DELETE CASCADE,
  token_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  revoked_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_svc_session_svc ON service_sessions(svc_id);
CREATE INDEX idx_svc_session_token ON service_sessions(token_hash_text);

-- Seed default capabilities
INSERT INTO capabilities (cap_id, cap_key) VALUES
  (uuid_generate_v4(), 'org.node:create'),
  (uuid_generate_v4(), 'org.node:read'),
  (uuid_generate_v4(), 'org.node:update'),
  (uuid_generate_v4(), 'org.node:deactivate'),
  (uuid_generate_v4(), 'org.assignment:create'),
  (uuid_generate_v4(), 'org.assignment:end'),
  (uuid_generate_v4(), 'org.assignment:read'),
  (uuid_generate_v4(), 'role:create'),
  (uuid_generate_v4(), 'role:read'),
  (uuid_generate_v4(), 'role:update'),
  (uuid_generate_v4(), 'role.capability:assign'),
  (uuid_generate_v4(), 'role.capability:revoke'),
  (uuid_generate_v4(), 'user:invite'),
  (uuid_generate_v4(), 'user:read'),
  (uuid_generate_v4(), 'user:update'),
  (uuid_generate_v4(), 'visibility:grant'),
  (uuid_generate_v4(), 'visibility:revoke'),
  (uuid_generate_v4(), 'audit:read');
