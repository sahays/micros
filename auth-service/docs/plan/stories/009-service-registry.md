# Story: Service Registry (Know-Your-Service)

Status: completed
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement KYS model for service-to-service authentication.

## Tasks

- [x] Create `models/service.rs` - Service, ServiceSecret
- [x] Create `models/service_permission.rs`
- [x] Create service repository
- [x] Implement Basic auth extraction middleware
- [x] Implement service token minting (optional)
- [x] Add admin endpoints for service management
- [x] Add permission checking middleware

## Service Auth Options

**Option A (Recommended):** Basic Auth
```
Authorization: Basic base64(svc_key:svc_secret)
```

**Option B:** Bearer Token (minted via `/svc/token`)
```
Authorization: Bearer <service_token>
```

## API Endpoints

```
POST /svc/register (admin only)
{
  "tenant_id": "uuid" | null,  // null = platform service
  "svc_key": "crm-bff",
  "svc_label": "CRM BFF",
  "permissions": ["authz.evaluate", "auth.context.read"]
}
Response:
{
  "svc_id": "uuid",
  "svc_key": "crm-bff",
  "svc_secret": "PLAINTEXT_RETURN_ONCE"
}

POST /svc/{svc_id}/secret/rotate
Authorization: Admin
Response:
{ "svc_secret": "PLAINTEXT_RETURN_ONCE" }

POST /svc/{svc_id}/disable
POST /svc/{svc_id}/enable

POST /svc/token (optional)
Authorization: Basic <svc_key:svc_secret>
Response:
{
  "service_token": "jwt_or_opaque",
  "expires_in": 3600
}

GET /svc/{svc_id}/permissions
POST /svc/{svc_id}/permissions
{ "perm_key": "authz.evaluate" }
DELETE /svc/{svc_id}/permissions/{perm_key}
```

## Permission Keys

- `authz.evaluate` - Can call POST /authz/evaluate
- `auth.context.read` - Can call GET /auth/context/{user_id}
- `org.read` - Can read org structure
- `user.read` - Can read user info
- `audit.read` - Can read audit logs

## Middleware

```rust
pub async fn require_service_auth(req: Request, next: Next) -> Response {
    // Extract Basic auth or Bearer token
    // Validate against service_secrets
    // Check service is active
    // Inject ServiceContext into request extensions
}

pub async fn require_service_permission(
    perm_key: &str,
) -> impl Fn(Request, Next) -> Future<Output = Response> {
    // Check ServiceContext has required permission
}
```

## Acceptance Criteria

- [x] Service registration returns secret once
- [x] Basic auth validates against hashed secret
- [x] Secret rotation works (old revoked)
- [x] Service can be disabled/enabled
- [x] Permission checking blocks unauthorized calls
- [x] /authz/evaluate requires "authz.evaluate" permission
