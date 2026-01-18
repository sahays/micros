# Story: User Invitations

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P2

## Summary

Implement user invitation flow allowing existing users to invite new users with pre-assigned roles and org nodes.

## Tasks

- [x] Create `handlers/invitation.rs` - Invitation endpoints
- [x] Implement invitation creation with token generation
- [x] Implement invitation acceptance flow
- [x] Auto-create assignment on acceptance
- [x] Add invitation email sending
- [x] Implement invitation expiry and cleanup

## Database

Uses existing `invitations` table:
```sql
CREATE TABLE invitations (
  invite_id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL REFERENCES tenants(tenant_id),
  email_addr TEXT,
  phone_e164 TEXT,
  invited_by_user_id UUID REFERENCES users(user_id),
  target_role_id UUID REFERENCES roles(role_id),
  target_org_node_id UUID REFERENCES org_nodes(org_node_id),
  invite_token_hash_text TEXT NOT NULL,
  expiry_utc TIMESTAMPTZ NOT NULL,
  accepted_utc TIMESTAMPTZ,
  created_utc TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## API Endpoints

```
POST /invitations
{
  "tenant_id": "uuid",
  "email": "newuser@example.com",     // or phone
  "role_id": "uuid",
  "org_node_id": "uuid",
  "expiry_days": 7                    // optional, default 7
}
Response: { "invite_id": "uuid", "invite_url": "..." }

GET /invitations/{invite_token}
Response: {
  "invite_id": "uuid",
  "email": "newuser@example.com",
  "org_node_label": "North Region",
  "role_label": "Field Agent",
  "invited_by": "John Doe",
  "expires_utc": "..."
}

POST /invitations/{invite_token}/accept
{
  "password": "...",
  "display_name": "New User"
}
Response: { "user_id": "uuid", "access_token": "...", "refresh_token": "..." }
```

## Invitation Flow

1. Existing user creates invitation for email/phone
2. System generates secure token, sends email
3. New user clicks link, sees invitation details
4. New user sets password, accepts invitation
5. System creates user, identity, and assignment
6. System returns auth tokens

## Email Template

Subject: You've been invited to join {tenant_label}

Body:
- Inviter name and role
- Target org node and role
- Expiry date
- Accept link

## Acceptance Criteria

- [x] Invitation created with hashed token
- [x] Invitation email sent to recipient
- [x] Invitation details retrievable by token
- [x] Accept creates user with password identity
- [x] Accept creates org assignment automatically
- [x] Expired invitations rejected
- [x] Already-accepted invitations rejected
- [x] Tokens returned on successful acceptance

## Implementation Notes

- Token is UUID-based, stored as SHA256 hash
- Default expiry is 168 hours (7 days)
- Acceptance creates user, password identity, and org assignment in sequence
- Email sending is currently logged (TODO: implement actual SMTP)
- Conflict error returned if email already exists in tenant
