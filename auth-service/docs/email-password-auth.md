# Email/Password Authentication Guide

This guide details the standard email and password authentication flows provided by the Auth Service.

## 1. Registration Flow

**Endpoint:** `POST /auth/register`

New users sign up with an email and password.

### Request
```json
{
  "email": "user@example.com",
  "password": "securePassword123",
  "name": "John Doe" // Optional
}
```

### Response
```json
{
  "user_id": "550e8400-...",
  "message": "Registration successful. Please check your email to verify your account."
}
```

**Process:**
1.  **Validation:** Email format check and password minimum length (8 chars).
2.  **Uniqueness:** Checks if email is already registered.
3.  **Hashing:** Hashes password using Argon2.
4.  **Verification:** Generates a secure token and emails a verification link to the user.
5.  **Audit Log:** Logs `user_registration` event.

## 2. Email Verification Flow

**Endpoint:** `GET /auth/verify?token=...`

Users must verify their email before logging in.

**Process:**
1.  User clicks link in email: `http://localhost:3000/auth/verify?token=...`
2.  **Validation:** Checks if token exists and is not expired.
3.  **Update:** Sets `verified: true` on the user record.
4.  **Cleanup:** Deletes the verification token.

## 3. Login Flow

**Endpoint:** `POST /auth/login`

Authenticate using credentials to obtain JWTs.

### Request
```json
{
  "email": "user@example.com",
  "password": "securePassword123"
}
```

### Response
```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

**Process:**
1.  **Verification:** Checks email exists and verifies password hash (constant-time).
2.  **Status Check:** Rejects login if email is not verified.
3.  **Token Issuance:** Generates Access Token (15 min) and Refresh Token (7 days).
4.  **Audit Log:** Logs `user_login` event.

## 4. Password Reset Flow

For users who forgot their password.

### Step 1: Request Reset
**Endpoint:** `POST /auth/password-reset/request`

```json
{ "email": "user@example.com" }
```
*   Sends an email with a reset link if the user exists.
*   Always returns `200 OK` to prevent email enumeration.

### Step 2: Confirm Reset
**Endpoint:** `POST /auth/password-reset/confirm`

```json
{
  "token": "reset_token_from_email",
  "new_password": "newSecurePassword123"
}
```
*   **Validation:** Checks token validity.
*   **Update:** Hashes new password and updates user record.
*   **Security:** **Invalidates all existing sessions (revokes refresh tokens).**

## 5. Password Change (Authenticated)

**Endpoint:** `POST /users/me/password`

For logged-in users to change their password.

**Headers:** `Authorization: Bearer <access_token>`

### Request
```json
{
  "current_password": "oldPassword123",
  "new_password": "newSecurePassword123"
}
```

**Process:**
1.  **Auth:** Verifies JWT access token.
2.  **Verify:** Checks `current_password` matches stored hash.
3.  **Update:** Updates password with `new_password`.
4.  **Security:** **Invalidates all other sessions.**

## 6. Security Features

*   **Argon2 Hashing:** Industry-standard memory-hard password hashing.
*   **Rate Limiting:**
    *   Login: 5 attempts / 15 min.
    *   Register: 3 attempts / 1 hour.
    *   Reset: 3 attempts / 1 hour.
*   **Session Management:**
    *   Short-lived Access Tokens (15 min).
    *   Rotated Refresh Tokens.
    *   Global logout capability (via token revocation).
