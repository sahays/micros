# Story: Authorization Evaluate Endpoint

Status: pending
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement `/authz/evaluate` endpoint for BFFs to check authorization decisions.

## Tasks

- [ ] Create `handlers/authz/evaluate.rs`
- [ ] Implement capability matching logic
- [ ] Implement org scope checking (own, subtree)
- [ ] Implement resource attribute matching
- [ ] Add service authentication requirement

## API Endpoint

```
POST /authz/evaluate
Authorization: Basic <svc_key:svc_secret> | Bearer <service_token>
Requires: service permission "authz.evaluate"

Request:
{
  "tenant_id": "uuid",
  "subject": {
    "user_id": "uuid",
    "assignment_id": "uuid"  // optional: evaluate specific assignment
  },
  "cap_key": "crm.visit:view:subtree",
  "resource": {
    "owner_user_id": "uuid",      // optional
    "org_node_id": "uuid",        // optional
    "attrs": { "key": "value" }   // optional extra attributes
  }
}

Response:
{
  "allow": true,
  "reason_key": "capability+subtree",
  "matched_assignment_id": "uuid",
  "matched_org_node_id": "uuid"
}

// Or denied:
{
  "allow": false,
  "reason_key": "no_matching_capability"
}
```

## Evaluation Logic

```
1. Load user's active assignments
2. For each assignment:
   a. Check if role has requested cap_key
   b. If cap_key ends with :own → check resource.owner_user_id == user_id
   c. If cap_key ends with :subtree → check resource.org_node_id in assignment's subtree
   d. If cap_key has no scope suffix → just check capability exists
3. Return first matching assignment or deny
```

## Reason Keys

- `capability_match` - Direct capability match
- `capability+own` - Capability with own-resource scope
- `capability+subtree` - Capability with subtree scope
- `no_matching_capability` - User lacks capability
- `out_of_scope` - Capability exists but resource outside scope
- `no_active_assignment` - User has no active assignments

## Acceptance Criteria

- [ ] Service-only endpoint (rejects user JWTs)
- [ ] Capability matching works
- [ ] :own scope checks owner_user_id
- [ ] :subtree scope uses closure table
- [ ] Returns structured allow/deny with reason
