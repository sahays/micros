# Story: Roles & Capabilities Registry

Status: pending
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement capability registry and tenant-scoped roles with capability mappings.

## Tasks

- [ ] Create `models/capability.rs` - Global capability registry
- [ ] Create `models/role.rs` - Tenant-scoped roles
- [ ] Create `models/role_capability.rs` - Role→capability mapping
- [ ] Seed default capabilities
- [ ] Create role repository
- [ ] Create capability repository
- [ ] Add role/capability API endpoints

## Capability Naming Convention

```
{domain}.{resource}:{action}[:scope]

Examples:
- org.node:create
- org.node:read
- org.node:update
- org.assignment:create
- crm.visit:create
- crm.visit:view:own
- crm.visit:view:subtree
- crm.visit:edit:own
```

## Default Capabilities (Seed)

```sql
INSERT INTO capabilities (cap_id, cap_key) VALUES
  (gen_random_uuid(), 'org.node:create'),
  (gen_random_uuid(), 'org.node:read'),
  (gen_random_uuid(), 'org.node:update'),
  (gen_random_uuid(), 'org.node:deactivate'),
  (gen_random_uuid(), 'org.assignment:create'),
  (gen_random_uuid(), 'org.assignment:end'),
  (gen_random_uuid(), 'org.assignment:read'),
  (gen_random_uuid(), 'role:create'),
  (gen_random_uuid(), 'role:read'),
  (gen_random_uuid(), 'role:update'),
  (gen_random_uuid(), 'user:invite'),
  (gen_random_uuid(), 'user:read'),
  (gen_random_uuid(), 'audit:read');
```

## API Endpoints

```
POST /capabilities
{ "cap_key": "crm.visit:create" }

GET /capabilities?prefix=crm.

POST /roles
{
  "tenant_id": "uuid",
  "role_label": "Regional Manager"
}

GET /roles?tenant_id=uuid

POST /roles/{role_id}/capabilities
{ "cap_id": "uuid" }

DELETE /roles/{role_id}/capabilities/{cap_id}

GET /roles/{role_id}/capabilities
```

## Acceptance Criteria

- [ ] Capabilities are globally unique by cap_key
- [ ] Roles are unique per tenant by role_label
- [ ] Role→capability mappings work
- [ ] Prefix search on capabilities works
- [ ] Default capabilities seeded on migration
