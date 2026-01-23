# Auth Service

**Multi-tenant authentication and authorization service with capability-based access control.**

## Problem

Every multi-tenant app needs user management, authentication, and authorization. Building custom auth per app leads to inconsistent security, duplicated effort, and permission sprawl. Hierarchical org structures with role-based access are complex to implement correctly.

## Solution

A reusable auth microservice providing:
- Multi-tenant user accounts with multiple identity providers
- JWT-based authentication with refresh token rotation
- Capability-based authorization with org hierarchy
- Time-bounded assignments and visibility grants

## Core Principles

- **Multi-tenant:** Complete isolation between tenants via tenant_id
- **Capability-based:** Fine-grained permissions, not just roles
- **Hierarchical:** Org tree with inheritance via closure table
- **Time-bounded:** Assignments have start/end, never deleted
- **Immutable audit:** All actions logged, no deletions

## Data Model

### Tenants
- `tenant_id`: UUID
- `tenant_slug`: unique identifier (e.g., "acme-corp")
- `tenant_label`: display name
- `tenant_state_code`: active, suspended

### Users
- `user_id`: UUID
- `tenant_id`: UUID (scoped to tenant)
- `email`: unique within tenant
- `email_verified`: boolean
- `google_id`: optional OAuth identifier
- `display_name`: optional
- `user_state_code`: active, suspended, deactivated

### User Identities
- `ident_id`: UUID
- `user_id`: UUID
- `ident_provider_code`: password, google
- `ident_hash`: hashed credential

### Org Nodes (Hierarchy)
- `org_node_id`: UUID
- `tenant_id`: UUID
- `node_type_code`: tenant-defined (e.g., "region", "branch", "team")
- `node_label`: display name
- `parent_org_node_id`: optional (null for root)
- `active_flag`: boolean

### Org Node Paths (Closure Table)
- `ancestor_org_node_id`: UUID
- `descendant_org_node_id`: UUID
- `depth_val`: integer (0 = self, 1 = direct child, etc.)

Enables efficient subtree queries: "all descendants of node X"

### Roles
- `role_id`: UUID
- `tenant_id`: UUID (tenant-scoped)
- `role_label`: display name

### Capabilities (Global)
- `cap_id`: UUID
- `cap_key`: permission string (e.g., "crm.visit:view:subtree")

**Capability Key Format:** `{domain}.{resource}:{action}[:scope]`
- Domain: logical grouping (crm, org, billing)
- Resource: entity type (visit, node, invoice)
- Action: operation (create, view, edit, delete)
- Scope: optional (own, subtree, all)

### Org Assignments
- `assignment_id`: UUID
- `tenant_id`: UUID
- `user_id`: UUID
- `org_node_id`: UUID
- `role_id`: UUID
- `start_utc`: when assignment begins
- `end_utc`: optional (null = active)

**Key insight:** Users are assigned to org nodes with roles. Roles grant capabilities. Assignments are time-bounded and never deleted—only ended.

### Visibility Grants
- `grant_id`: UUID
- `user_id`: UUID
- `org_node_id`: UUID
- `access_scope_code`: read, analyze

Allows users to see org nodes outside their assigned subtree (e.g., executives viewing all branches).

## gRPC Services

### AuthService
| Method | Description |
|--------|-------------|
| `Register` | Create user with email/password |
| `Login` | Authenticate with email/password |
| `Refresh` | Exchange refresh token for new tokens |
| `Logout` | Revoke refresh token |
| `ValidateToken` | Validate access token, return claims |
| `SendOtp` | Send OTP via email/SMS/WhatsApp |
| `VerifyOtp` | Verify OTP code |

### OrgService
| Method | Description |
|--------|-------------|
| `CreateOrgNode` | Create org node in hierarchy |
| `GetOrgNode` | Get org node by ID |
| `GetOrgNodeDescendants` | Get all descendants of a node |
| `ListTenantOrgNodes` | List all org nodes for tenant |
| `GetTenantOrgTree` | Get full org tree structure |

### RoleService
| Method | Description |
|--------|-------------|
| `CreateRole` | Create tenant-scoped role |
| `GetRole` | Get role by ID |
| `ListTenantRoles` | List roles for tenant |
| `GetRoleCapabilities` | Get capabilities assigned to role |
| `AssignCapability` | Add capability to role |
| `ListCapabilities` | List all capabilities |
| `GetCapability` | Get capability by key |

### AssignmentService
| Method | Description |
|--------|-------------|
| `CreateAssignment` | Assign user to org node with role |
| `EndAssignment` | End assignment (set end_utc) |
| `ListUserAssignments` | List user's active assignments |

### AuthzService
| Method | Description |
|--------|-------------|
| `GetAuthContext` | Get user's full auth context |
| `CheckCapability` | Check if user has specific capability |

### VisibilityService
| Method | Description |
|--------|-------------|
| `CreateVisibilityGrant` | Grant cross-subtree visibility |
| `RevokeVisibilityGrant` | Revoke visibility grant |
| `ListUserVisibilityGrants` | List user's visibility grants |

### InvitationService
| Method | Description |
|--------|-------------|
| `CreateInvitation` | Invite user to org node with role |
| `GetInvitation` | Get invitation by token |
| `AcceptInvitation` | Accept invitation, create user |

### AuditService
| Method | Description |
|--------|-------------|
| `ListAuditEvents` | List audit events with filters |

## Authorization Flow

1. **Login:** User authenticates → receives JWT access + refresh tokens
2. **Request:** Client sends access token in Authorization header
3. **Validate:** Service validates token, extracts claims (user_id, tenant_id)
4. **Context:** Service calls `GetAuthContext` to get full permissions
5. **Check:** Service calls `CheckCapability` for specific operations
6. **Audit:** Action logged to audit_events table

## Auth Context Response

```json
{
  "user_id": "uuid",
  "tenant_id": "uuid",
  "assignments": [
    {
      "org_node_id": "uuid",
      "role_id": "uuid",
      "capabilities": ["crm.visit:view:subtree", "crm.visit:edit:own"]
    }
  ],
  "visibility_grants": [
    {
      "org_node_id": "uuid",
      "access_scope": "read"
    }
  ]
}
```

## Capability Scopes

| Scope | Meaning |
|-------|---------|
| (none) | Global within tenant |
| `own` | Only resources created by user |
| `subtree` | Resources in assigned org node and descendants |

**Example:** User with `crm.visit:view:subtree` at "North Region" can view visits for North Region and all child branches.

## Use Cases

- **Multi-tenant SaaS:** Each customer is a tenant with isolated users
- **Hierarchical orgs:** Regions → Branches → Teams with inherited access
- **Field apps:** Sales reps see their assigned territory, managers see all

## Key Features

- **JWT RS256:** Access tokens signed with RSA, refresh tokens hashed in DB
- **Token rotation:** New refresh token on each refresh (revocation possible)
- **OTP:** Email/SMS verification for passwordless or 2FA
- **Google OAuth:** Social login with automatic user creation
- **Invitations:** Email-based user onboarding with pre-assigned roles
- **Audit trail:** Immutable log of all auth and permission changes

## Integration Pattern

```
Client App
    │
    │ Authorization: Bearer <access_token>
    ▼
API Gateway / BFF
    │
    │ ValidateToken() or GetAuthContext()
    ▼
Auth Service
    │
    │ JWT validation, permission lookup
    ▼
PostgreSQL (users, roles, assignments)
Redis (token blacklist)
```

## Edge Cases

- **Duplicate email:** Rejected (unique per tenant)
- **Expired token:** Returns 401, client refreshes
- **Revoked refresh token:** Returns 401, user must re-login
- **Inactive user:** Login rejected
- **Suspended tenant:** All users blocked
- **Overlapping assignments:** Allowed (union of capabilities)
- **Deleted org node:** Soft delete (active_flag = false)

## Non-Goals

- User profile management (use domain service)
- Password policies (basic validation only)
- Session management beyond tokens
- Rate limiting (handled by gateway)
- Service-to-service auth (internal services have unrestricted access)
- SAML/LDAP (future consideration)

## Capabilities

Capabilities control access to auth-service operations. Each capability maps to specific service actions.

**Format:** `{domain}.{resource}:{action}[:scope]`

### Org Node Management

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `org.node:create` | OrgService.CreateOrgNode | Create org nodes in hierarchy |
| `org.node:read` | OrgService.GetOrgNode, GetOrgNodeDescendants, ListTenantOrgNodes, GetTenantOrgTree | View org node details and hierarchy |
| `org.node:update` | OrgService.UpdateOrgNode | Modify org node label or type |
| `org.node:deactivate` | OrgService.DeactivateOrgNode | Soft-delete org node (set active_flag=false) |

### Org Assignments

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `org.assignment:create` | AssignmentService.CreateAssignment | Assign user to org node with role |
| `org.assignment:read` | AssignmentService.ListUserAssignments | View user's org assignments |
| `org.assignment:end` | AssignmentService.EndAssignment | End an assignment (set end_utc) |

### Role Management

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `role:create` | RoleService.CreateRole | Create tenant-scoped roles |
| `role:read` | RoleService.GetRole, ListTenantRoles, GetRoleCapabilities | View role details and capabilities |
| `role:update` | RoleService.UpdateRole | Modify role label |
| `role.capability:assign` | RoleService.AssignCapability | Add capability to role |
| `role.capability:revoke` | RoleService.RevokeCapability | Remove capability from role |

### Capability Registry

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `capability:read` | RoleService.ListCapabilities, GetCapability | View available capabilities |

### User Management

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `user:read` | UserService.GetUser, ListUsers | View user details |
| `user:update` | UserService.UpdateUser | Modify user display name or state |
| `user:invite` | InvitationService.CreateInvitation | Invite new users to org |

### Invitation Management

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `invitation:read` | InvitationService.GetInvitation, ListInvitations | View pending invitations |
| `invitation:revoke` | InvitationService.RevokeInvitation | Cancel pending invitation |

### Visibility Grants

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `visibility:grant` | VisibilityService.CreateVisibilityGrant | Grant cross-subtree visibility |
| `visibility:read` | VisibilityService.ListUserVisibilityGrants | View user's visibility grants |
| `visibility:revoke` | VisibilityService.RevokeVisibilityGrant | Revoke visibility grant |

### Audit

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `audit:read` | AuditService.ListAuditEvents | View audit trail |

### Tenant Management

| Capability | Service Action | Description |
|------------|----------------|-------------|
| `tenant:read` | TenantService.GetTenant | View tenant details |
| `tenant:update` | TenantService.UpdateTenant | Modify tenant label or state |

### Public Endpoints (No Capability Required)

These endpoints are accessible without capabilities:

| Service | Methods | Description |
|---------|---------|-------------|
| AuthService | Register, Login, Refresh, Logout | User authentication |
| AuthService | SendOtp, VerifyOtp | OTP verification |
| AuthService | ValidateToken | Token validation |
| AuthzService | GetAuthContext, CheckCapability | Authorization context (self) |
| InvitationService | AcceptInvitation | Accept invitation (invitee) |

### Default Seeded Capabilities

The following capabilities are seeded on first migration:

```sql
org.node:create, org.node:read, org.node:update, org.node:deactivate
org.assignment:create, org.assignment:read, org.assignment:end
role:create, role:read, role:update
role.capability:assign, role.capability:revoke
capability:read
user:read, user:update, user:invite
invitation:read, invitation:revoke
visibility:grant, visibility:read, visibility:revoke
audit:read
tenant:read, tenant:update
```
