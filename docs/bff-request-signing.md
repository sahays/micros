# BFF Request Signing & Security Guide

This guide details the request signing mechanism used to secure communication between the Backend-for-Frontend (BFF) and the Authentication Service.

## Overview

To prevent request tampering and replay attacks, all state-changing requests from the BFF to the Auth Service must be signed using HMAC-SHA256. This ensures:
- **Integrity**: The request body and metadata have not been altered.
- **Authenticity**: The request originated from a trusted client (BFF) possessing the signing secret.
- **Freshness**: The request is recent (timestamp check) and unique (nonce check).

## Headers

The following headers are required for signed requests:

| Header | Description |
|--------|-------------|
| `X-Client-ID` | The unique identifier of the client (BFF). |
| `X-Timestamp` | Unix timestamp (seconds) of when the request was created. |
| `X-Nonce` | A unique random string (at least 16 chars) for this request. |
| `X-Signature` | The Hex-encoded HMAC-SHA256 signature. |

## Signing Process

To generate the signature:

1.  **Construct the Payload**:
    Concatenate the following fields with a pipe `|` separator:
    `METHOD|PATH|TIMESTAMP|NONCE|BODY_HASH`

    - `METHOD`: HTTP method (e.g., "POST").
    - `PATH`: Request path (e.g., "/auth/login").
    - `TIMESTAMP`: The value of `X-Timestamp`.
    - `NONCE`: The value of `X-Nonce`.
    - `BODY_HASH`: SHA-256 hash of the request body (Hex-encoded). If body is empty, hash of empty string.

2.  **Calculate HMAC**:
    Compute the HMAC-SHA256 of the payload using the `signing_secret` (NOT the `client_secret` used for OAuth).

3.  **Hex Encode**:
    Convert the binary HMAC result to a hexadecimal string.

### Example (Rust)

```rust
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(
    secret: &str,
    method: &str,
    path: &str,
    timestamp: i64,
    nonce: &str,
    body: &str,
) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Invalid key");
    let body_hash = hex::encode(Sha256::digest(body.as_bytes()));
    
    let payload = format!("{}|{}|{}|{}|{}", method, path, timestamp, nonce, body_hash);
    
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
```

### Example (Node.js)

```javascript
const crypto = require('crypto');

function generateSignature(secret, method, path, timestamp, nonce, body) {
    const bodyHash = crypto.createHash('sha256').update(body || '').digest('hex');
    const payload = `${method}|${path}|${timestamp}|${nonce}|${bodyHash}`;
    
    const hmac = crypto.createHmac('sha256', secret);
    hmac.update(payload);
    return hmac.digest('hex');
}
```

## Security Controls

- **Timestamp Validation**: Requests older than 60 seconds are rejected.
- **Replay Prevention**: Nonces are tracked for 120 seconds. A reused nonce within this window causes a 401 error.
- **Constant-Time Verification**: The server uses constant-time comparison to prevent timing attacks.

## Secret Rotation

The `signing_secret` is distinct from the `client_secret` used for OAuth token exchange. It can be rotated via the Admin API:

`POST /auth/admin/clients/:client_id/rotate`

This endpoint returns a `new_signing_secret` which should be immediately updated in the BFF configuration.
