# Epic: Auth-Service v2.0 - PostgreSQL Migration & Capability-Based AuthZ

Status: completed
Created: 2026-01-18

## Overview

Complete rewrite of auth-service from MongoDB to PostgreSQL with:
- Capability-based authorization (not role labels)
- Know-Your-Service (KYS) model for service-to-service auth
- Org node hierarchy with closure table
- Time-bounded immutable assignments

## Auth Philosophy

### Core Principles

1. **Separation of concerns**
   - Auth-service owns: identity, org hierarchy, role→capability mapping, auth evaluation
   - BFFs/domain services own: business semantics, workflows, validations

2. **Never authorize by role label**
   - Roles are opaque literals created by tenants
   - Authorization based on: capability strings, org scope, resource attributes

3. **Time-bound truth**
   - Memberships/responsibilities via immutable, time-bounded assignments
   - Never rewrite history; only end assignments and create new

4. **Know Your Service**
   - Every BFF/domain service registered with `svc_key` + `svc_secret`
   - Service-to-service calls authenticated independently from end-users

## Database Schema

PostgreSQL with these tables:
- `tenants` - Multi-tenant root
- `users` - User accounts per tenant
- `user_identities` - Auth providers (password, google)
- `refresh_sessions` - Token sessions
- `otp_codes` - One-time passwords
- `org_nodes` - Org hierarchy nodes
- `org_node_paths` - Closure table for hierarchy
- `roles` - Tenant-defined roles
- `capabilities` - Global capability registry
- `role_capabilities` - Role→capability mapping
- `org_assignments` - User assignments to org nodes with roles
- `visibility_grants` - Cross-org visibility
- `invitations` - User invites
- `audit_events` - Immutable audit log
- `services` - Registered BFFs/services (KYS)
- `service_secrets` - Service credentials
- `service_permissions` - Service API access control
- `service_sessions` - Optional service tokens

## Stories

### Phase 1 (Foundational) - COMPLETED

- [x] [001-postgres-setup](../stories/001-postgres-setup.md) - PostgreSQL connection, migrations, sqlx
- [x] [002-tenant-user-model](../stories/002-tenant-user-model.md) - Tenants, users, identities
- [x] [003-password-auth](../stories/003-password-auth.md) - Signup, login, refresh, logout
- [x] [004-org-hierarchy](../stories/004-org-hierarchy.md) - Org nodes, closure table, tree ops
- [x] [005-roles-capabilities](../stories/005-roles-capabilities.md) - Role/capability registry
- [x] [006-org-assignments](../stories/006-org-assignments.md) - User→org→role assignments
- [x] [007-auth-context](../stories/007-auth-context.md) - GET /auth/context endpoint
- [x] [008-authz-evaluate](../stories/008-authz-evaluate.md) - POST /authz/evaluate endpoint
- [x] [009-service-registry](../stories/009-service-registry.md) - KYS: service registration, secrets

### Phase 2 - COMPLETED

- [x] [010-otp-auth](../stories/010-otp-auth.md) - OTP send/verify (email, SMS)
- [x] [011-google-oauth](../stories/011-google-oauth.md) - Google login integration
- [x] [012-visibility-grants](../stories/012-visibility-grants.md) - Cross-org visibility
- [x] [013-invitations](../stories/013-invitations.md) - User invitation flow
- [x] [014-audit-events](../stories/014-audit-events.md) - Audit logging

## API Routes

### Service Auth (KYS)
- `POST /svc/register` - Register service (admin)
- `POST /svc/{svc_id}/secret/rotate` - Rotate secret
- `POST /svc/token` - Mint service token (optional)

### User Auth
- `POST /auth/signup`
- `POST /auth/login/password`
- `POST /auth/otp/send`
- `POST /auth/otp/verify`
- `POST /auth/login/google`
- `POST /auth/token/refresh`
- `POST /auth/logout`

### Context & Authorization
- `GET /auth/context` - User's auth context
- `GET /auth/context/{user_id}` - Service fetches user context
- `POST /authz/evaluate` - Authorization decision

### Org Management
- `POST /org/nodes`
- `PATCH /org/nodes/{org_node_id}`
- `POST /org/nodes/{org_node_id}/deactivate`
- `GET /org/nodes/{org_node_id}`
- `GET /org/nodes/{org_node_id}/children`
- `GET /org/tree`

### Assignments
- `POST /org/assignments`
- `POST /org/assignments/{assignment_id}/end`
- `GET /users/{user_id}/assignments`
- `GET /org/nodes/{org_node_id}/users`

### Roles & Capabilities
- `POST /roles`
- `GET /roles`
- `POST /capabilities`
- `GET /capabilities`
- `POST /roles/{role_id}/capabilities`
- `DELETE /roles/{role_id}/capabilities/{cap_id}`

### Visibility & Invites
- `POST /org/visibility-grants`
- `POST /org/visibility-grants/{grant_id}/revoke`
- `POST /auth/users/invite`
- `POST /auth/invitations/accept`

### Audit
- `GET /audit/events`

## Acceptance Criteria

- [x] All MongoDB dependencies removed
- [x] PostgreSQL with sqlx for type-safe queries
- [x] Database migrations via sqlx-cli
- [x] All existing tests adapted or replaced
- [x] JWT tokens preserved (RS256)
- [x] Service authentication via Basic auth or Bearer token
- [x] Capability-based authorization working
- [x] Org hierarchy with subtree queries
- [x] Time-bounded assignments
- [x] Audit trail for all mutations
