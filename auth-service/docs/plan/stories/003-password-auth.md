# Story: Password Authentication

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P0

## Summary

Implement email/password signup, login, refresh, and logout flows.

## Tasks

- [x] Create `handlers/auth/signup.rs` - User registration
- [x] Create `handlers/auth/login.rs` - Password login
- [x] Create `handlers/auth/refresh.rs` - Token refresh
- [x] Create `handlers/auth/logout.rs` - Session revocation
- [x] Adapt JWT service for new user model
- [x] Create refresh_sessions repository
- [x] Password hashing with argon2

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

- [x] Signup creates user + password identity
- [x] Login validates password, returns tokens
- [x] Refresh rotates tokens, invalidates old
- [x] Logout revokes refresh session
- [x] Password hashed with argon2id
- [x] Refresh token stored as hash only
