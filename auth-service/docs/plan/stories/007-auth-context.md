# Story: Auth Context Endpoint

Status: pending
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement `/auth/context` endpoint returning user's full authorization context.

## Tasks

- [ ] Create `handlers/auth/context.rs`
- [ ] Build context aggregation query
- [ ] Return assignments, roles, capabilities, visibility grants
- [ ] Add service-authenticated variant for BFFs

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

- [ ] User can fetch own context with JWT
- [ ] Service can fetch any user's context with service auth
- [ ] Capabilities aggregated from all active assignments
- [ ] Visibility grants included
- [ ] Response cached briefly (optional)
