# Story: PostgreSQL Setup & Migration Infrastructure

Status: pending
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Replace MongoDB with PostgreSQL using sqlx for compile-time checked queries and sqlx-cli for migrations.

## Tasks

- [ ] Add sqlx, sqlx-cli dependencies to Cargo.toml
- [ ] Remove mongodb dependency
- [ ] Create DATABASE_URL config
- [ ] Set up connection pool (PgPool)
- [ ] Create migrations folder structure
- [ ] Write initial migration with all tables
- [ ] Update docker-compose with PostgreSQL service
- [ ] Create health check endpoint using Postgres
- [ ] Update .env.example with Postgres config

## Schema (Migration 001)

```sql
-- 001_initial_schema.sql

-- Tenants
CREATE TABLE tenants (
  tenant_id UUID PRIMARY KEY,
  tenant_label TEXT NOT NULL,
  tenant_state_code TEXT NOT NULL CHECK (tenant_state_code IN ('active','suspended')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Users
CREATE TABLE users (
  user_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  email_addr TEXT,
  phone_e164 TEXT,
  display_label TEXT,
  user_state_code TEXT NOT NULL CHECK (user_state_code IN ('active','inactive')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(tenant_id, email_addr),
  UNIQUE(tenant_id, phone_e164)
);

-- User Identities
CREATE TABLE user_identities (
  ident_id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(user_id),
  ident_provider_code TEXT NOT NULL CHECK (ident_provider_code IN ('password','google')),
  provider_subject_key TEXT,
  email_verified_flag BOOLEAN NOT NULL DEFAULT FALSE,
  phone_verified_flag BOOLEAN NOT NULL DEFAULT FALSE,
  password_hash_text TEXT,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(ident_provider_code, provider_subject_key),
  UNIQUE(user_id, ident_provider_code)
);

-- Refresh Sessions
CREATE TABLE refresh_sessions (
  session_id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(user_id),
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  refresh_token_hash_text TEXT NOT NULL,
  client_user_agent_text TEXT,
  client_ip_text TEXT,
  expiry_utc TIMESTAMPTZ NOT NULL,
  revoked_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_refresh_user ON refresh_sessions(user_id);

-- OTP Codes
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
CREATE INDEX idx_otp_dest ON otp_codes(destination_text);

-- Org Nodes
CREATE TABLE org_nodes (
  org_node_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  node_type_code TEXT NOT NULL,
  node_label TEXT NOT NULL,
  parent_org_node_id UUID REFERENCES org_nodes(org_node_id),
  active_flag BOOLEAN NOT NULL DEFAULT TRUE,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_org_nodes_tenant ON org_nodes(tenant_id);
CREATE INDEX idx_org_nodes_parent ON org_nodes(parent_org_node_id);

-- Org Node Paths (Closure Table)
CREATE TABLE org_node_paths (
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  ancestor_org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  descendant_org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  depth_val INT NOT NULL,
  PRIMARY KEY (ancestor_org_node_id, descendant_org_node_id)
);
CREATE INDEX idx_org_path_ancestor ON org_node_paths(ancestor_org_node_id);
CREATE INDEX idx_org_path_desc ON org_node_paths(descendant_org_node_id);

-- Roles
CREATE TABLE roles (
  role_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  role_label TEXT NOT NULL,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(tenant_id, role_label)
);

-- Capabilities (Global)
CREATE TABLE capabilities (
  cap_id UUID PRIMARY KEY,
  cap_key TEXT NOT NULL UNIQUE,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Role Capabilities
CREATE TABLE role_capabilities (
  role_id UUID NOT NULL REFERENCES roles(role_id),
  cap_id UUID NOT NULL REFERENCES capabilities(cap_id),
  PRIMARY KEY (role_id, cap_id)
);

-- Org Assignments
CREATE TABLE org_assignments (
  assignment_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  user_id UUID NOT NULL REFERENCES users(user_id),
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  role_id UUID NOT NULL REFERENCES roles(role_id),
  start_utc TIMESTAMPTZ NOT NULL,
  end_utc TIMESTAMPTZ,
  CHECK (end_utc IS NULL OR end_utc > start_utc)
);
CREATE INDEX idx_assign_user ON org_assignments(user_id);
CREATE INDEX idx_assign_node ON org_assignments(org_node_id);

-- Visibility Grants
CREATE TABLE visibility_grants (
  grant_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  user_id UUID NOT NULL REFERENCES users(user_id),
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  access_scope_code TEXT NOT NULL CHECK (access_scope_code IN ('read','analyze')),
  start_utc TIMESTAMPTZ NOT NULL,
  end_utc TIMESTAMPTZ
);
CREATE INDEX idx_vis_user ON visibility_grants(user_id);

-- Invitations
CREATE TABLE invitations (
  invite_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  email_addr TEXT,
  phone_e164 TEXT,
  invited_by_user_id UUID REFERENCES users(user_id),
  target_role_id UUID REFERENCES roles(role_id),
  target_org_node_id UUID REFERENCES org_nodes(org_node_id),
  invite_token_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  accepted_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Audit Events
CREATE TABLE audit_events (
  audit_id UUID PRIMARY KEY,
  tenant_id UUID REFERENCES tenants(tenant_id),
  actor_user_id UUID REFERENCES users(user_id),
  action_key TEXT NOT NULL,
  entity_kind_code TEXT NOT NULL,
  entity_id UUID,
  occurred_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  payload_json JSONB
);
CREATE INDEX idx_audit_tenant_time ON audit_events(tenant_id, occurred_utc DESC);

-- Services (KYS)
CREATE TABLE services (
  svc_id UUID PRIMARY KEY,
  tenant_id UUID REFERENCES tenants(tenant_id),
  svc_key TEXT NOT NULL UNIQUE,
  svc_label TEXT NOT NULL,
  svc_state_code TEXT NOT NULL CHECK (svc_state_code IN ('active','disabled')),
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Service Secrets
CREATE TABLE service_secrets (
  secret_id UUID PRIMARY KEY,
  svc_id UUID NOT NULL REFERENCES services(svc_id),
  secret_hash_text TEXT NOT NULL,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_utc TIMESTAMPTZ
);
CREATE INDEX idx_svc_secret_svc ON service_secrets(svc_id);

-- Service Permissions
CREATE TABLE service_permissions (
  svc_id UUID NOT NULL REFERENCES services(svc_id),
  perm_key TEXT NOT NULL,
  PRIMARY KEY (svc_id, perm_key)
);

-- Service Sessions
CREATE TABLE service_sessions (
  svc_session_id UUID PRIMARY KEY,
  svc_id UUID NOT NULL REFERENCES services(svc_id),
  token_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  revoked_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## Acceptance Criteria

- [ ] `cargo sqlx prepare` generates offline query data
- [ ] `sqlx migrate run` applies migrations
- [ ] Connection pool configured with env vars
- [ ] Health check queries database
- [ ] Docker Compose includes PostgreSQL 16
