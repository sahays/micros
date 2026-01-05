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
    -   Defines HTTP endpoints using Axum.
    -   Handles request deserialization and response serialization.
    -   Performs initial input validation.
2.  **Middleware Layer:**
    -   **Security:** Request signing verification, Headers (CORS, HSTS).
    -   **Traffic Control:** Distributed rate limiting via Redis (Governor).
    -   **Observability:** Request tracing and ID propagation.
3.  **Service Layer:**
    -   Contains business logic (e.g., `JwtService`, `EmailService`).
    -   Orchestrates operations between data access and external providers.
4.  **Data Access Layer (Models/DB):**
    -   Type-safe interactions with MongoDB.
    -   Defines data models (Users, Clients, Tokens).
    -   Handles Redis interactions for ephemeral state.

**State Management:**
Configuration and database connections are encapsulated in a thread-safe `AppState` struct, injected into handlers via Axum's `State` extractor.

## Security-First Architecture

The service is built on a "Zero Trust" and "Defense in Depth" philosophy.

### 1. The BFF Pattern (Backend-for-Frontend)
Instead of exposing the Auth Service directly to browsers, we encourage using a **BFF**.
*   **Isolation:** The BFF acts as a trusted proxy.
*   **Request Signing:** The BFF signs requests using a shared secret (`signing_secret`), ensuring that no malicious actor can bypass the frontend logic or replay requests.
*   **Client Registry:** Only registered BFFs (Known Clients) can interact with the system, enforced via `client_id` checks.

### 2. Service Accounts & Scopes
Internal communication follows **Least Privilege**:
*   **No Implicit Trust:** Being on the internal network is not enough.
*   **Scoped Access:** A "Billing Service" can be restricted to `user:read` but denied `user:write`.
*   **Audit Trails:** Every service-to-service call is cryptographically tied to a Service ID and logged for compliance.

## Quick Start

### 1. Setup Environment
```bash
cp .env.example .env
mkdir -p keys
openssl genrsa -out keys/private.pem 2048
openssl rsa -in keys/private.pem -pubout -out keys/public.pem
```

### 2. Run with Docker
```bash
docker build -t auth-service .
docker run -p 3000:3000 --env-file .env auth-service
```

### 3. Run with Docker Compose (Full Stack)

To run the Auth Service alongside MongoDB, Redis, and the PLG Observability stack:

```bash
docker-compose up -d
```

**Port Configuration:**
To avoid port conflicts on your host machine, you can customize the exposed ports using environment variables in a `.env` file (in the root directory) or by exporting them:

```bash
export AUTH_SERVICE_PORT=8081
export REDIS_PORT=6380
export MONGO_PORT=27018
docker-compose up -d
```

Default ports:
- Auth Service: 8080
- MongoDB: 27017
- Redis: 6379
- Grafana: 3000
- Prometheus: 9090
- Loki: 3100
- Promtail: 9080

## Usage Guide

### Authentication
- **Register**: `POST /auth/register` (Email/Password)
- **Login**: `POST /auth/login` returns `access_token` and `refresh_token`.
- **Refresh**: `POST /auth/refresh` rotates both tokens.
- **Logout**: `POST /auth/logout` blacklists the current session.

### User Management
- **Profile**: `GET /users/me` (Requires Bearer Token)
- **Update**: `PATCH /users/me` (Name/Email)
- **Security**: `POST /users/me/password` to change password.

### Service-to-Service
- **App Token**: `POST /auth/app/token` using `client_id` and `client_secret`.
- **Introspection**: `POST /auth/introspect` to verify token validity.

### Administration
- **Client Management**: `POST /auth/admin/clients` to register new apps (BFF/Mobile).
- **Service Accounts**: `POST /auth/admin/services` for backend integrations.
- **Audit Logs**: `GET /auth/admin/services/{id}/audit-log`.

## API Documentation

Access the interactive documentation at:
- **Swagger UI**: `http://localhost:3000/docs`
- **Spec**: `http://localhost:3000/.well-known/openapi.json`

## Detailed Documentation

- [Email/Password Auth Guide](docs/email-password-auth.md): Registration, login, and password management flows.
- [Security Controls & Defenses](docs/security-controls.md): Rate limiting, bot protection, and client registries.
- [BFF Request Signing Guide](docs/bff-request-signing.md): Implementation details for securing Frontend-to-Backend communication.
- [Service Integration Guide](docs/service-integration.md): How to authenticate other microservices with Auth Service.
- [Social Login Guide](docs/social-login.md): Google OAuth 2.0 integration details.
- [Audit Logging](docs/audit-logging.md): Events, schema, and admin access.
- [Observability & PLG Stack](docs/observability.md): Structured logging, tracing, and PLG integration.

## Deployment

Production deployments are handled via `scripts/deploy.sh`, which implements atomic release switching and health-check driven rollbacks.