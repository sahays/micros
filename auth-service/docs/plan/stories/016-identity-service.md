# Story: Identity Service

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P0

## Summary

Implement core IdentityService gRPC service with resolution, linking, and query operations.

## Tasks

- [ ] Create proto definition `proto/micros/auth/v1/identity.proto`
- [ ] Implement `ResolveIdentity` RPC
- [ ] Implement `AddAttribute` RPC
- [ ] Implement `LinkUserToIdentity` RPC
- [ ] Implement `GetLinkedUsers` RPC
- [ ] Implement `GetIdentity` RPC
- [ ] Add database queries for identity operations
- [ ] Register service in gRPC server

## gRPC Methods

### ResolveIdentity
Find or create identity for a user based on attributes.

**Input:** user_id, attributes[]
**Output:** identity_id, is_new, match_confidence, matched_attributes[]

**Logic:**
1. Search existing identities by normalized attributes
2. Exact match on verified attribute → link to existing
3. No match → create new identity
4. Return identity with confidence score

### AddAttribute
Add attribute to an identity.

**Input:** identity_id, attribute_type, attribute_value
**Output:** attribute_id, normalized_value, verification_status

### LinkUserToIdentity
Manually link user to identity.

**Input:** user_id, identity_id, link_method, confidence
**Output:** success

### GetLinkedUsers
Get all users linked to an identity.

**Input:** identity_id
**Output:** users[] (user_id, tenant_id, email, link_confidence, link_method)

### GetIdentity
Get identity with all attributes.

**Input:** identity_id
**Output:** identity, attributes[], linked_user_count

## Acceptance Criteria

- [ ] Proto compiles and generates Rust code
- [ ] ResolveIdentity finds existing identity by email match
- [ ] ResolveIdentity creates new identity when no match
- [ ] AddAttribute stores normalized value
- [ ] LinkUserToIdentity creates link record
- [ ] GetLinkedUsers returns all tenant users
- [ ] All operations logged to audit_events
