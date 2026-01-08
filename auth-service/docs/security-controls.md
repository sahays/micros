# Security Controls & Defenses

This document outlines the multi-layered security controls implemented in the Auth Service, focusing on Rate Limiting, Bot Detection, and Known Client management.

## 1. Rate Limiting Strategy

We utilize the **Token Bucket** algorithm (via the `governor` crate) to enforce fair usage and prevent abuse.

### Layers of Defense

1.  **Global IP Limiter:**
    *   **Scope:** All incoming requests.
    *   **Limit:** `100 requests / 60 seconds` (Default).
    *   **Purpose:** Prevents general DoS attacks and scanning.

2.  **Endpoint-Specific Limiters:**
    *   **Login:** `5 attempts / 15 minutes`.
    *   **Registration:** `3 attempts / 1 hour`.
    *   **Password Reset:** `3 attempts / 1 hour`.
    *   **Purpose:** Stops credential stuffing, brute-force attacks, and spam sign-ups.

3.  **Client-Specific Limiter (BFF/App):**
    *   **Scope:** Requests carrying a valid `X-Client-ID`.
    *   **Limit:** Configurable per client (e.g., `1000 requests / minute`).
    *   **Unlimited Access:** Set `rate_limit_per_min = 0` for trusted service clients (e.g., BFF) to bypass rate limiting.
    *   **Purpose:** Allows trusted clients higher throughput while containing compromised keys. Service clients making server-to-server calls can be configured with unlimited access.

### Response Headers

When a limit is exceeded, the service returns `429 Too Many Requests` with:

| Header | Description |
|--------|-------------|
| `Retry-After` | Seconds to wait before retrying. |
| `X-RateLimit-Limit` | The quota limit. |
| `X-RateLimit-Remaining` | Remaining requests in the window. |

## 2. Bot & Abuse Detection

Bot detection middleware analyzes request patterns to identify and block automated traffic.

### Detection Heuristics

*   **Known Bot User-Agents:** Blocks requests from known bot signatures (using `isbot` crate).
*   **Missing Browser Headers:** Flags requests claiming to be browsers but missing standard headers (Accept, Accept-Language, Accept-Encoding).
*   **Empty User-Agent:** Suspicious for public endpoints.

### Exemptions

*   **Service-to-Service Calls:** Requests with `X-Signature` header (signed requests from registered clients) are exempt from bot detection.
*   **Health/Metrics Endpoints:** `/health` and `/metrics` are excluded.
*   **CORS Preflight:** `OPTIONS` requests are excluded.

### Abuse Prevention

*   **Credential Stuffing:** Blocked by the strict 5-attempt limit on `/auth/login`.
*   **Account Enumeration:**
    *   `/auth/login`: Returns generic "Invalid credentials" messages.
    *   `/auth/password-reset/request`: Always returns `200 OK` regardless of email existence.
*   **Spam Registration:** Blocked by the 3-attempt limit on `/auth/register`.

**Future Enhancements:**
*   CAPTCHA integration (Turnstile/ReCAPTCHA) for flagged IPs.
*   Enhanced fingerprinting and behavioral analysis.

## 3. Known Clients & Access Control

The service operates on a "Strict Registry" model. Only known, registered entities can interact with protected APIs.

### Backend-for-Frontend (BFF) Clients

*   **Registration:** Admin-only via `POST /auth/admin/clients`.
*   **Credentials:**
    *   `client_id`: Public identifier.
    *   `client_secret`: Used for `POST /auth/app/token` (App Identity).
    *   `signing_secret`: Used to sign requests (HMAC-SHA256).
*   **Security:**
    *   **Request Signing:** Critical state-changing requests *must* be signed. This ensures requests originate from the trusted BFF server, not a malicious script acting as the user.
    *   **Secret Rotation:** Supported without downtime via `POST .../rotate`.

### Service Accounts

*   **Registration:** Admin-only via `POST /auth/admin/services`.
*   **Credentials:** `api_key` (Prefix: `svc_live_` or `svc_test_`).
*   **Scope Enforcement:**
    *   Services are granted specific scopes (e.g., `user:read`).
    *   Middleware validates that the API key has the required scope for the endpoint.

### Access Hierarchy

1.  **Public:** `/.well-known/*`, `/auth/login`, `/auth/register` (Protected by IP Rate Limit).
2.  **Client-Authenticated:** `/auth/app/token` (Requires Client Credentials).
3.  **User-Authenticated:** `/users/me` (Requires Bearer JWT).
4.  **Service-Authenticated:** Internal APIs (Requires Scoped API Key).
5.  **Admin-Authenticated:** `/auth/admin/*` (Requires Admin Master Key).
