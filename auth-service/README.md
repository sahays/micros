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

## Deployment

Production deployments are handled via `scripts/deploy.sh`, which implements atomic release switching and health-check driven rollbacks.