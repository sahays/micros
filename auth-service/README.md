# Auth Service v2

Multi-tenant authentication and authorization service with capability-based access control.

## Features

- **Multi-tenant**: Isolated tenants with org node hierarchies (closure table)
- **Capability-based AuthZ**: Fine-grained permissions, never authorize by role label
- **Time-bounded assignments**: Immutable role assignments with start/end dates
- **Multiple auth methods**: Email/password, OTP (email/SMS), Google OAuth
- **Cross-org visibility**: Grants for users to see outside their subtree
- **Know-Your-Service (KYS)**: Service registry with scoped permissions
- **BFF support**: HMAC request signing for secure frontend communication
- **Observability**: JSON logging, OpenTelemetry traces, Prometheus metrics

## Tech Stack

- **Framework**: Axum 0.7 + Tokio
- **Database**: PostgreSQL (sqlx with compile-time checked queries)
- **Cache**: Redis (token blacklist, rate limiting)
- **Auth**: RS256 JWTs, Argon2 passwords, Google OAuth 2.0
- **Observability**: tracing + OpenTelemetry → Tempo, JSON logs → Loki

## Quick Start

```bash
# Prerequisites: PostgreSQL and Redis running locally

# Generate JWT keys
mkdir -p keys
openssl genrsa -out keys/private.pem 2048
openssl rsa -in keys/private.pem -pubout -out keys/public.pem

# Set environment
export DATABASE_URL="postgresql://user:pass@localhost/auth_db"
export REDIS_URL="redis://localhost:6379"
export JWT_PRIVATE_KEY_PATH="keys/private.pem"
export JWT_PUBLIC_KEY_PATH="keys/public.pem"

# Run
cargo run -p auth-service
```

**Dev Stack**: `./scripts/dev-up.sh` (port 9005)
**Prod Stack**: `./scripts/prod-up.sh` (port 10005)

## API Overview

| Endpoint | Description |
|----------|-------------|
| `POST /auth/register` | Register user with email/password |
| `POST /auth/login` | Login, returns JWT pair |
| `POST /auth/refresh` | Refresh access token |
| `POST /auth/logout` | Revoke refresh token |
| `POST /auth/otp/send` | Send OTP via email/SMS/WhatsApp |
| `POST /auth/otp/verify` | Verify OTP, returns JWT pair |
| `GET /auth/google` | Initiate Google OAuth |
| `POST /auth/google/token` | Exchange Google ID token |
| `GET /auth/context` | Get user's auth context (roles, capabilities) |
| `POST /authz/evaluate` | Evaluate capability check |
| `POST /orgs` | Create org node |
| `POST /roles` | Create role with capabilities |
| `POST /assignments` | Assign user to role at org node |
| `POST /visibility-grants` | Grant cross-org visibility |
| `POST /invitations` | Invite user with pre-assigned role |
| `GET /audit/events` | Query audit log |
| `POST /services` | Register service (KYS) |

## BFF Integration

### 1. Register Your BFF

```bash
curl -X POST http://localhost:9005/auth/admin/clients \
  -H "X-Admin-Api-Key: $ADMIN_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "app_name": "my-frontend",
    "app_type": "service",
    "rate_limit_per_min": 0
  }'
# Save: client_id and signing_secret
```

### 2. Sign Requests (Rust Example)

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn sign_request(
    method: &str,
    path: &str,
    body: &str,
    client_id: &str,
    signing_secret: &str,
) -> (String, String, String) {
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();

    let string_to_sign = format!("{method}{path}{timestamp}{nonce}{body}");

    let mut mac = Hmac::<Sha256>::new_from_slice(signing_secret.as_bytes()).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    (timestamp, nonce, signature)
}

// Usage with reqwest
async fn call_auth_service(client: &reqwest::Client) -> Result<Response, Error> {
    let body = r#"{"email":"user@example.com","password":"secret"}"#;
    let (ts, nonce, sig) = sign_request("POST", "/auth/login", body, CLIENT_ID, SIGNING_SECRET);

    client.post("http://auth-service:3000/auth/login")
        .header("Content-Type", "application/json")
        .header("X-Client-ID", CLIENT_ID)
        .header("X-Timestamp", ts)
        .header("X-Nonce", nonce)
        .header("X-Signature", sig)
        .body(body)
        .send()
        .await
}
```

### 3. TypeScript/Node.js Example

```typescript
import crypto from 'crypto';

function signRequest(
  method: string,
  path: string,
  body: string,
  clientId: string,
  signingSecret: string
): { timestamp: string; nonce: string; signature: string } {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const nonce = crypto.randomUUID();

  const stringToSign = `${method}${path}${timestamp}${nonce}${body}`;
  const signature = crypto
    .createHmac('sha256', signingSecret)
    .update(stringToSign)
    .digest('hex');

  return { timestamp, nonce, signature };
}

// Usage with fetch
async function login(email: string, password: string) {
  const body = JSON.stringify({ email, password });
  const { timestamp, nonce, signature } = signRequest(
    'POST', '/auth/login', body, CLIENT_ID, SIGNING_SECRET
  );

  return fetch('http://auth-service:3000/auth/login', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Client-ID': CLIENT_ID,
      'X-Timestamp': timestamp,
      'X-Nonce': nonce,
      'X-Signature': signature,
    },
    body,
  });
}
```

### 4. Common BFF Flows

```typescript
// Login flow
const loginResponse = await login(email, password);
const { access_token, refresh_token } = await loginResponse.json();
// Store tokens in HTTP-only cookies (BFF pattern)

// Get auth context for UI
const context = await callAuthService('GET', '/auth/context', '', access_token);
// Returns: { user_id, tenant_id, assignments: [{ org_node_id, role_label, capabilities }] }

// Check capability before action
const canApprove = await callAuthService('POST', '/authz/evaluate', JSON.stringify({
  user_id: userId,
  capability_key: 'invoice:approve',
  org_node_id: targetOrgId,
}));

// OTP login (passwordless)
await callAuthService('POST', '/auth/otp/send', JSON.stringify({
  tenant_id: tenantId,
  destination: email,
  channel: 'email',
  purpose: 'login',
}));
// User receives code, then:
const otpResponse = await callAuthService('POST', '/auth/otp/verify', JSON.stringify({
  otp_id: otpId,
  code: userEnteredCode,
}));
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | required |
| `REDIS_URL` | Redis connection string | required |
| `JWT_PRIVATE_KEY_PATH` | RS256 private key | required |
| `JWT_PUBLIC_KEY_PATH` | RS256 public key | required |
| `OTLP_ENDPOINT` | Tempo endpoint for traces | optional |
| `LOG_LEVEL` | Logging level | `info` |
| `ADMIN_API_KEY` | Admin API key | required |

## Observability

- **Logs**: JSON to stdout, collected by Promtail → Loki
- **Traces**: OTLP to Tempo (set `OTLP_ENDPOINT`)
- **Metrics**: `/metrics` endpoint for Prometheus
- **Health**: `GET /health` (checks PostgreSQL + Redis)

Query logs in Grafana:
```
{container=~".*auth-service"} | json | level="error"
```

## Documentation

- [BFF Request Signing](docs/bff-request-signing.md)
- [Security Controls](docs/security-controls.md)
- [Observability](docs/observability.md)
