# BFF Request Signing & Security Guide

This guide details the request signing mechanism used to secure communication between a Backend-for-Frontend (BFF) and the Authentication Service.

## 1. Setup & Credentials

Before implementing signing, you must register your BFF client to obtain credentials.

**Create Client (Admin Only):**
```bash
POST /auth/admin/clients
X-Admin-Api-Key: <ADMIN_API_KEY_FROM_ENV>
Content-Type: application/json

{
  "app_name": "My BFF",
  "app_type": "web",
  "rate_limit_per_min": 1000,
  "allowed_origins": ["https://my-bff.com"]
}
```

**Response:**
```json
{
  "client_id": "...",
  "signing_secret": "use-this-for-signing",
  ...
}
```

> **Note:** Store `signing_secret` securely. It is **distinct** from the OAuth `client_secret`.

## 2. Required Headers

All state-changing requests (POST, PUT, PATCH, DELETE) from the BFF must include:

| Header | Description |
|--------|-------------|
| `X-Client-ID` | The `client_id` received during registration. |
| `X-Timestamp` | Current Unix timestamp (seconds). |
| `X-Nonce` | Unique random string (min 16 chars). |
| `X-Signature` | Hex-encoded HMAC-SHA256 signature. |

## 3. Signing Algorithm

**Formula:**
`HMAC_SHA256(key=signing_secret, data=payload) -> HexString`

**Payload Construction:**
`METHOD|PATH|TIMESTAMP|NONCE|BODY_HASH`

- `METHOD`: HTTP verb (e.g., `POST`).
- `PATH`: Request path (e.g., `/auth/login`).
- `TIMESTAMP`: Value of `X-Timestamp`.
- `NONCE`: Value of `X-Nonce`.
- `BODY_HASH`: `SHA256(RequestBody) -> HexString`. (Hash of empty string if no body).

## 4. Implementation (Pseudocode)

```python
function sign_request(secret, method, path, body):
    timestamp = current_unix_time()
    nonce = random_string(16)
    
    # 1. Hash Body
    body_hash = hex(sha256(body))
    
    # 2. Build Payload
    payload = f"{method}|{path}|{timestamp}|{nonce}|{body_hash}"
    
    # 3. Sign
    signature = hex(hmac_sha256(key=secret, msg=payload))
    
    return {
        "X-Client-ID": CLIENT_ID,
        "X-Timestamp": timestamp,
        "X-Nonce": nonce,
        "X-Signature": signature
    }
```

## 5. Security Constraints

- **Timestamp Validity**: +/- 60 seconds.
- **Nonce Replay**: Tracked for 120 seconds.
- **Key Rotation**: Use `POST /auth/admin/clients/{id}/rotate` to rotate secrets without downtime.