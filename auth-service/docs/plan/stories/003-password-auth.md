# Story: Password Authentication

Status: pending
Epic: [001-auth-service-v2](../epics/001-auth-service-v2.md)
Priority: P0

## Summary

Implement email/password signup, login, refresh, and logout flows.

## Tasks

- [ ] Create `handlers/auth/signup.rs` - User registration
- [ ] Create `handlers/auth/login.rs` - Password login
- [ ] Create `handlers/auth/refresh.rs` - Token refresh
- [ ] Create `handlers/auth/logout.rs` - Session revocation
- [ ] Adapt JWT service for new user model
- [ ] Create refresh_sessions repository
- [ ] Password hashing with argon2

## API Endpoints

```
POST /auth/signup
{
  "tenant_id": "uuid",
  "email_addr": "user@example.com",
  "password": "...",
  "display_label": "John Doe"
}
Response: { "user_id": "uuid", "access_token": "...", "refresh_token": "..." }

POST /auth/login/password
{
  "tenant_id": "uuid",
  "email_addr": "user@example.com",
  "password": "..."
}
Response: { "access_token": "...", "refresh_token": "...", "expires_in": 900 }

POST /auth/token/refresh
{
  "refresh_token": "..."
}
Response: { "access_token": "...", "refresh_token": "...", "expires_in": 900 }

POST /auth/logout
Authorization: Bearer <access_token>
Response: 204 No Content
```

## JWT Claims

```rust
pub struct AccessTokenClaims {
    pub sub: String,        // user_id
    pub tenant_id: String,
    pub email: Option<String>,
    pub iat: i64,
    pub exp: i64,
}
```

## Acceptance Criteria

- [ ] Signup creates user + password identity
- [ ] Login validates password, returns tokens
- [ ] Refresh rotates tokens, invalidates old
- [ ] Logout revokes refresh session
- [ ] Password hashed with argon2id
- [ ] Refresh token stored as hash only
