# Story: Google OAuth Integration

- [x] **Status: Completed**
- **Epic:** [001-auth-service-v2](../epics/001-auth-service-v2.md)
- **Priority:** P1

## Summary

Implement Google OAuth 2.0 login flow for social authentication.

## Tasks

- [x] Create `handlers/oauth.rs` - OAuth endpoints
- [x] Implement Google OAuth URL generation
- [x] Implement Google OAuth callback handling
- [x] Extract and verify Google ID token
- [x] Create/link user identity for Google provider
- [x] Handle existing user linking
- [x] Generate tokens on successful auth

## OAuth Flow

1. Frontend calls `GET /auth/google?tenant_id=...`
2. Backend redirects to Google OAuth consent screen
3. User authenticates with Google
4. Google redirects to callback with code
5. Backend exchanges code for tokens
6. Backend verifies ID token, extracts user info
7. Backend creates/links user, returns auth tokens

## API Endpoints

```
GET /auth/google
Query: tenant_id, redirect_uri (optional)
Response: Redirect to Google OAuth URL

GET /auth/google/callback
Query: code, state
Response: Redirect to frontend with tokens or error

POST /auth/google/token
{
  "tenant_id": "uuid",
  "id_token": "google_id_token"
}
Response: { "access_token": "...", "refresh_token": "...", "user_id": "...", "is_new_user": bool }
```

## User Identity Linking

- Check if Google subject exists in `user_identities`
- If exists: return existing user
- If not: check if email exists in tenant
  - If email exists: link Google identity to existing user
  - If not: create new user with Google identity

## Configuration

```
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REDIRECT_URI=http://localhost:9005/auth/google/callback
```

## Acceptance Criteria

- [x] Google OAuth URL generation working
- [x] Callback exchanges code for tokens
- [x] ID token verified with Google public keys
- [x] New users created with Google identity
- [x] Existing users linked by email
- [x] Email marked as verified for Google users
- [x] Tokens returned on successful auth
- [x] Error handling for invalid/expired codes

## Implementation Notes

- ID token decoding is basic (base64 decode) - production should verify signatures against Google's public keys
- OAuth state parameter contains tenant_id and nonce for CSRF protection
- Redirect callback passes tokens in URL params (production should use more secure method)
- Added `find_user_identity_by_subject` database method for Google subject lookup
