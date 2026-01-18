# Story: Org Assignments

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P0

## Summary

Implement time-bounded user→org→role assignments.

## Tasks

- [x] Create `models/org_assignment.rs`
- [x] Create assignment repository
- [x] Implement assignment creation with start_utc
- [x] Implement assignment ending (set end_utc)
- [x] Query active assignments for user
- [x] Query users at org node with role filter
- [x] Add assignment API endpoints

## Key Concepts

- Assignments are **immutable** - never update, only end
- `end_utc = NULL` means currently active
- Time-bounded: `start_utc <= now() < end_utc`
- A user can have multiple assignments (different orgs/roles)

## Queries

```sql
-- Active assignments for user
SELECT * FROM org_assignments
WHERE user_id = $1
  AND start_utc <= now()
  AND (end_utc IS NULL OR end_utc > now());

-- Users at org node (including subtree) with role
SELECT DISTINCT a.user_id, a.role_id, a.org_node_id
FROM org_assignments a
JOIN org_node_paths p ON a.org_node_id = p.descendant_org_node_id
WHERE p.ancestor_org_node_id = $1
  AND ($2::uuid IS NULL OR a.role_id = $2)
  AND a.start_utc <= now()
  AND (a.end_utc IS NULL OR a.end_utc > now());

-- End assignment
UPDATE org_assignments
SET end_utc = now()
WHERE assignment_id = $1 AND end_utc IS NULL;
```

## API Endpoints

```
POST /org/assignments
{
  "tenant_id": "uuid",
  "user_id": "uuid",
  "org_node_id": "uuid",
  "role_id": "uuid",
  "start_utc": "2026-01-18T00:00:00Z"  // optional, defaults to now
}
Response: { "assignment_id": "uuid" }

POST /org/assignments/{assignment_id}/end
Response: 204 No Content

GET /users/{user_id}/assignments?active=true
Response: [{ assignment_id, org_node_id, role_id, start_utc, end_utc }]

GET /org/nodes/{org_node_id}/users?active=true&role_id=uuid&include_subtree=true
Response: [{ user_id, role_id, org_node_id, assignment_id }]
```

## Acceptance Criteria

- [x] Assignments created with start_utc
- [x] Assignments ended by setting end_utc
- [x] Active query filters by time correctly
- [x] Subtree user query uses closure table
- [x] No assignment mutation (only create/end)
