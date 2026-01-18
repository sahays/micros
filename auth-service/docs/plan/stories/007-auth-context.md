# Story: Auth Context Endpoint

Status: completed
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement `/auth/context` endpoint returning user's full authorization context.

## Tasks

- [x] Create `handlers/auth/context.rs`
- [x] Build context aggregation query
- [x] Return assignments, roles, capabilities, visibility grants
- [x] Add service-authenticated variant for BFFs

## API Endpoints

```
GET /auth/context
Authorization: Bearer <user_access_token>

Response:
{
  "user_id": "uuid",
  "tenant_id": "uuid",
  "email_addr": "user@example.com",
  "display_label": "John Doe",
  "assignments": [
    {
      "assignment_id": "uuid",
      "org_node_id": "uuid",
      "org_node_label": "North Region",
      "role_id": "uuid",
      "role_label": "Regional Manager",
      "capabilities": ["org.node:read", "crm.visit:view:subtree"]
    }
  ],
  "visibility_grants": [
    {
      "grant_id": "uuid",
      "org_node_id": "uuid",
      "access_scope_code": "read"
    }
  ]
}

GET /auth/context/{user_id}
Authorization: Basic <svc_key:svc_secret> | Bearer <service_token>
Requires: service permission "auth.context.read"
Response: Same as above
```

## Acceptance Criteria

- [x] User can fetch own context with JWT
- [x] Service can fetch any user's context with service auth
- [x] Capabilities aggregated from all active assignments
- [x] Visibility grants included
- [x] Response cached briefly (optional)
