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
- [BFF Request Signing Guide](docs/bff-request-signing.md): Implementation details for securing Frontend-to-Backend communication.
- [Service Integration Guide](docs/service-integration.md): How to authenticate other microservices with Auth Service.
- [Social Login Guide](docs/social-login.md): Google OAuth 2.0 integration details.
- [Audit Logging](docs/audit-logging.md): Events, schema, and admin access.
- [Observability & GCP](docs/observability.md): Structured logging, tracing, and Google Cloud integration.

## Deployment

Production deployments are handled via `scripts/deploy.sh`, which implements atomic release switching and health-check driven rollbacks.