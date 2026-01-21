# Story: Admin Tools

- [ ] **Status: Planning**
- **Epic:** [002-identity-resolution](../epics/002-identity-resolution.md)
- **Priority:** P2

## Summary

Implement admin operations for identity management: merge, split, flag, and audit.

## Tasks

- [ ] Implement `MergeIdentities` RPC
- [ ] Implement `SplitIdentity` RPC
- [ ] Implement `FlagIdentity` RPC
- [ ] Implement `ListFlaggedIdentities` RPC
- [ ] Add admin permission checks
- [ ] Create detailed audit trail for admin actions

## gRPC Methods

### MergeIdentities
Combine two identities into one (e.g., duplicate detected).

**Input:** source_identity_id, target_identity_id, reason
**Output:** success, merged_identity_id

**Logic:**
1. Move all attributes from source to target
2. Relink all users from source to target
3. Mark source as merged (soft delete)
4. Audit with full before/after state

### SplitIdentity
Separate users from an identity (e.g., wrongly linked).

**Input:** identity_id, user_ids_to_split[], reason
**Output:** new_identity_id

**Logic:**
1. Create new identity
2. Relink specified users to new identity
3. Copy relevant attributes to new identity
4. Audit with full details

### FlagIdentity
Mark identity for review (e.g., suspected fraud).

**Input:** identity_id, flag_type, reason
**Output:** success

**Flag Types:**
- duplicate_suspected
- fraud_suspected
- verification_failed
- manual_review_required

### ListFlaggedIdentities
Get identities requiring admin attention.

**Input:** flag_type (optional), limit, offset
**Output:** identities[] with flag details

## Permissions

| Operation | Required Permission |
|-----------|---------------------|
| MergeIdentities | identity.admin.merge |
| SplitIdentity | identity.admin.split |
| FlagIdentity | identity.admin.flag |
| ListFlaggedIdentities | identity.admin.read |
| SearchIdentities | identity.search |

## Audit Requirements

All admin actions must log:
- Actor (admin user_id)
- Action performed
- Before state (identities, links, attributes)
- After state
- Reason provided
- Timestamp

## Acceptance Criteria

- [ ] Merge combines attributes and user links
- [ ] Merge preserves all verification statuses
- [ ] Split creates independent identity
- [ ] Split reassigns specified users only
- [ ] Flag marks identity with reason
- [ ] Flagged list filterable by type
- [ ] All operations require admin permission
- [ ] Full audit trail for compliance
- [ ] Merge/split reversible via counter-operation
