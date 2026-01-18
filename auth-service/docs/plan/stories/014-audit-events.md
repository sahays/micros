# Story: Audit Events

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P1

## Summary

Implement comprehensive audit logging for all security-relevant mutations and access events.

## Tasks

- [x] Create `handlers/audit.rs` - Audit query endpoint
- [x] Implement audit event insertion helper
- [x] Add audit logging to all auth handlers
- [x] Add audit logging to org/role/assignment handlers
- [x] Add audit logging to service handlers
- [x] Implement audit query with filters

## Database

Uses existing `audit_events` table:
```sql
CREATE TABLE audit_events (
  audit_id UUID PRIMARY KEY,
  tenant_id UUID REFERENCES tenants(tenant_id),
  actor_user_id UUID REFERENCES users(user_id),
  action_key TEXT NOT NULL,
  entity_kind_code TEXT NOT NULL,
  entity_id UUID,
  occurred_utc TIMESTAMPTZ NOT NULL DEFAULT now(),
  payload_json JSONB
);
```

## Event Types

Authentication:
- `user_registered`
- `user_login_success`
- `user_login_failed`
- `user_logout`
- `token_refreshed`
- `password_changed`
- `otp_sent`
- `otp_verified`
- `otp_failed`

Organization:
- `org_node_created`
- `org_node_updated`
- `org_node_deactivated`
- `assignment_created`
- `assignment_ended`
- `visibility_grant_created`
- `visibility_grant_revoked`

Roles & Capabilities:
- `role_created`
- `role_updated`
- `capability_assigned`
- `capability_revoked`

Service (KYS):
- `service_registered`
- `service_secret_rotated`
- `service_disabled`
- `service_enabled`

Invitations:
- `invitation_created`
- `invitation_accepted`
- `invitation_expired`

## API Endpoints

```
GET /audit/events
Query:
  - tenant_id (required)
  - actor_user_id (optional)
  - action_key (optional)
  - entity_kind (optional)
  - entity_id (optional)
  - from_utc (optional)
  - to_utc (optional)
  - limit (optional, default 100, max 1000)
  - offset (optional)
Response: {
  "events": [...],
  "total": 1234,
  "limit": 100,
  "offset": 0
}
```

## Audit Helper

```rust
impl Database {
    pub async fn log_audit_event(
        &self,
        tenant_id: Option<Uuid>,
        actor_user_id: Option<Uuid>,
        action_key: &str,
        entity_kind: &str,
        entity_id: Option<Uuid>,
        payload: Option<serde_json::Value>,
    ) -> Result<(), sqlx::Error>;
}
```

## Sensitive Data Handling

- Never log passwords, tokens, or secrets
- Log hashed identifiers where appropriate
- Include IP address and user agent for auth events
- Payload should contain relevant context (old/new values for updates)

## Acceptance Criteria

- [x] All auth events logged (login, logout, register, etc.)
- [x] All org mutations logged
- [x] All role/capability changes logged
- [x] All service operations logged
- [x] Audit query endpoint working with filters
- [x] Pagination working correctly
- [x] No sensitive data in audit logs
- [x] IP address captured for auth events

## Implementation Notes

- AuditEvent model has factory methods: user_action(), service_action(), system_action()
- Query endpoint supports multiple filters: tenant, actor, action, entity kind/id, date range
- Pagination with limit (max 1000) and offset
- Returns total count for pagination UI
- Dynamic SQL building for flexible filtering
