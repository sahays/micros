# Story: Visibility Grants

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P2

## Summary

Implement cross-org visibility grants allowing users to view data from org nodes outside their assignment hierarchy.

## Tasks

- [x] Create `handlers/visibility.rs` - Visibility grant endpoints
- [x] Implement grant creation with time bounds
- [x] Implement grant revocation
- [x] Add visibility to auth context response
- [x] Integrate visibility into authz evaluation

## Database

Uses existing `visibility_grants` table:
```sql
CREATE TABLE visibility_grants (
  grant_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  user_id UUID NOT NULL REFERENCES users(user_id),
  org_node_id UUID NOT NULL REFERENCES org_nodes(org_node_id),
  access_scope_code TEXT NOT NULL CHECK (access_scope_code IN ('read','analyze')),
  start_utc TIMESTAMPTZ NOT NULL,
  end_utc TIMESTAMPTZ
);
```

## API Endpoints

```
POST /visibility-grants
{
  "tenant_id": "uuid",
  "user_id": "uuid",
  "org_node_id": "uuid",
  "access_scope": "read" | "analyze",
  "start_utc": "2026-01-18T00:00:00Z",  // optional, defaults to now
  "end_utc": "2026-12-31T23:59:59Z"     // optional, null = indefinite
}
Response: { "grant_id": "uuid" }

POST /visibility-grants/{grant_id}/revoke
Response: 204 No Content

GET /users/{user_id}/visibility-grants?active=true
Response: [{ grant_id, org_node_id, access_scope_code, start_utc, end_utc }]
```

## Access Scopes

- `read`: View data from the org node and its subtree
- `analyze`: Read + aggregate/report on data

## Auth Context Integration

The `/auth/context` response includes visibility grants:
```json
{
  "visibility_grants": [
    {
      "grant_id": "uuid",
      "org_node_id": "uuid",
      "org_node_label": "East Region",
      "access_scope_code": "read"
    }
  ]
}
```

## AuthZ Integration

When evaluating authorization:
1. Check assignment-based access first
2. If denied, check visibility grants for the resource's org node
3. Visibility grants only provide read/analyze, not write capabilities

## Acceptance Criteria

- [x] Visibility grants created with time bounds
- [x] Grants revoked by setting end_utc
- [x] Active grants returned in auth context
- [x] AuthZ evaluation considers visibility grants
- [x] Visibility scope correctly limits access type
- [x] Grants respect org node hierarchy (subtree access)

## Implementation Notes

- Updated VisibilityGrant model to include access_scope_code, start_utc, end_utc
- Added AccessScope enum (Read, Analyze)
- Added `is_active()` method for time-bound checking
- Added database methods: find_active_visibility_grants_for_user, find_visibility_grant_by_id, revoke_visibility_grant
- Revocation sets end_utc to NOW() rather than deleting
