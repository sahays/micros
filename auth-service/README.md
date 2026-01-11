# Auth Service

A high-performance, secure authentication microservice built with Axum and MongoDB.

## Features

- **Multi-tenant Auth**: Support for Web, Mobile, and Service-to-Service clients.
- **Secure Storage**: Argon2 password hashing and encrypted secrets.
- **JWT Lifecycle**: RS256 token issuance with rotation, revocation, and blacklisting.
- **Social Login**: Google OAuth 2.0 integration with PKCE.
- **BFF Support**: HMAC-based request signing for tamper-proof communication.
- **Observability**: Structured JSON logging, health checks, and audit trails.
- **Documentation**: Automatic OpenAPI 3.0 generation and interactive Swagger UI.

## Tech Stack

- **Language:** Rust (2021 Edition)
- **Web Framework:** [Axum 0.7](https://github.com/tokio-rs/axum) - Ergonomic and modular web framework.
- **Runtime:** [Tokio](https://tokio.rs/) - Asynchronous runtime.
- **Database:** [MongoDB](https://www.mongodb.com/) - Primary data store for users and logs.
- **Caching/State:** [Redis](https://redis.io/) - Used for token blacklisting and rate limiting state.
- **Authentication:**
  - `jsonwebtoken` for RS256 JWT handling.
  - `argon2` for secure password hashing.
  - `oauth2` / `reqwest` for Social Login flows.
- **Documentation:** `utoipa` - Code-first OpenAPI/Swagger generation.
- **Infrastructure:** Docker (Multistage builds based on `debian:bookworm-slim`).

## Architecture

The service follows a **Layered Architecture** to ensure separation of concerns and testability:

1.  **Transport Layer (Handlers):**
    - Defines HTTP endpoints using Axum.
    - Handles request deserialization and response serialization.
    - Performs initial input validation.
2.  **Middleware Layer:**
    - **Security:** Request signing verification, Headers (CORS, HSTS).
    - **Traffic Control:** Distributed rate limiting via Redis (Governor).
    - **Observability:** Request tracing and ID propagation.
3.  **Service Layer:**
    - Contains business logic (e.g., `JwtService`, `EmailService`).
    - Orchestrates operations between data access and external providers.
4.  **Data Access Layer (Models/DB):**
    - Type-safe interactions with MongoDB.
    - Defines data models (Users, Clients, Tokens).
    - Handles Redis interactions for ephemeral state.

**State Management:** Configuration and database connections are encapsulated in a thread-safe `AppState` struct,
injected into handlers via Axum's `State` extractor.

## Security-First Architecture

The service is built on a "Zero Trust" and "Defense in Depth" philosophy.

### 1. The BFF Pattern (Backend-for-Frontend)

Instead of exposing the Auth Service directly to browsers, we encourage using a **BFF**.

- **Isolation:** The BFF acts as a trusted proxy.
- **Request Signing:** The BFF signs requests using a shared secret (`signing_secret`), ensuring that no malicious actor
  can bypass the frontend logic or replay requests.
- **Client Registry:** Only registered BFFs (Known Clients) can interact with the system, enforced via `client_id`
  checks.

### 2. Service Accounts & Scopes

Internal communication follows **Least Privilege**:

- **No Implicit Trust:** Being on the internal network is not enough.
- **Scoped Access:** A "Billing Service" can be restricted to `user:read` but denied `user:write`.
- **Audit Trails:** Every service-to-service call is cryptographically tied to a Service ID and logged for compliance.

## Quick Start

**Prerequisites:** MongoDB and Redis running on host machine (port 27017 and 6379).

```bash
# From repository root:
./scripts/dev-up.sh              # Start dev stack
./scripts/dev-down.sh            # Stop dev stack
./scripts/dev-up.sh --rebuild    # Rebuild and start
```

**Access Points (Dev):**

- Auth Service: http://localhost:9005
- Swagger UI: http://localhost:9005/docs
- Grafana: http://localhost:9002 (admin/admin)
- Prometheus: http://localhost:9000

**Production Stack:**

```bash
./scripts/prod-up.sh             # Everything containerized (ports 10000-10009)
./scripts/prod-down.sh
```

## Usage Workflows

### 1. User Registration & Login

```bash
# Register new user
curl -X POST http://localhost:9005/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "name": "John Doe"
  }'

# Login (returns access_token and refresh_token)
curl -X POST http://localhost:9005/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'

# Access protected endpoint
curl -X GET http://localhost:9005/users/me \
  -H "Authorization: Bearer <access_token>"

# Refresh tokens before expiry
curl -X POST http://localhost:9005/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "<refresh_token>"}'

# Logout (blacklist tokens)
curl -X POST http://localhost:9005/auth/logout \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "<refresh_token>"}'
```

### 2. Register BFF/Frontend Client

```bash
# Register secure-frontend as a service client (unlimited rate limits)
curl -X POST http://localhost:9005/auth/admin/clients \
  -H "X-Admin-Api-Key: <ADMIN_API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "app_name": "secure-frontend",
    "app_type": "service",
    "rate_limit_per_min": 0,
    "allowed_origins": ["http://localhost:9006"]
  }'

# Save client_id and signing_secret to .env.dev:
# APP_AUTH_SERVICE__CLIENT_ID=<client_id>
# APP_AUTH_SERVICE__SIGNING_SECRET=<signing_secret>
```

### 3. Service-to-Service Authentication

```bash
# Get app token using client credentials
curl -X POST http://localhost:9005/auth/app-token \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "<client_id>",
    "client_secret": "<client_secret>"
  }'

# Verify token validity
curl -X POST http://localhost:9005/auth/introspect \
  -H "Content-Type: application/json" \
  -d '{"token": "<access_token>"}'
```

### 4. Password Reset Flow

```bash
# Request password reset (sends email)
curl -X POST http://localhost:9005/auth/password-reset/request \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'

# Confirm reset with token from email
curl -X POST http://localhost:9005/auth/password-reset/confirm \
  -H "Content-Type: application/json" \
  -d '{
    "token": "<reset_token>",
    "new_password": "NewSecurePass123!"
  }'
```

## API Documentation

Interactive Swagger UI (auto-generated from code):

- **Dev**: http://localhost:9005/docs
- **Prod**: http://localhost:10005/docs (disabled by default)
- **OpenAPI Spec**: http://localhost:9005/.well-known/openapi.json

**Adding New Endpoints:**

1. Add `#[utoipa::path]` annotation to handler
2. Register in `ApiDoc` struct (src/lib.rs:31-96)
3. Rebuild - spec updates automatically

## Detailed Documentation

- [Email/Password Auth Guide](docs/email-password-auth.md): Registration, login, and password management flows.
- [Security Controls & Defenses](docs/security-controls.md): Rate limiting, bot protection, and client registries.
- [BFF Request Signing Guide](docs/bff-request-signing.md): Implementation details for securing Frontend-to-Backend
  communication.
- [Service Integration Guide](docs/service-integration.md): How to authenticate other microservices with Auth Service.
- [Social Login Guide](docs/social-login.md): Google OAuth 2.0 integration details.
- [Audit Logging](docs/audit-logging.md): Events, schema, and admin access.
- [Observability & PLG Stack](docs/observability.md): Structured logging, tracing, and PLG integration.

## Development Workflow

```bash
# Make code changes
vim src/handlers/auth/session.rs

# Run tests
cargo test -p auth-service

# Rebuild and restart
./scripts/dev-up.sh --rebuild

# View logs
docker-compose -f docker-compose.dev.yml logs -f auth-service

# Test endpoint
curl http://localhost:9005/health
```

**Pre-commit Hooks:**

- Runs `cargo fmt`, `cargo clippy`, and tests automatically
- Install: `ln -s ../../scripts/pre-commit.sh .git/hooks/pre-commit`

## Deployment

**Development:** `./scripts/dev-up.sh` (MongoDB/Redis on host, services in Docker) **Production:**
`./scripts/prod-up.sh` (everything containerized)

Deployment scripts include health checks and graceful shutdown handling.
