# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Micros** is a production-ready microservices monorepo implementing secure, observable Rust-based services with full PLG (Prometheus, Loki, Grafana) + Tempo observability stack.

**Stack:**
- Backend API: Rust/Axum (auth-service)
- Web Frontend: Rust/Axum/Htmx (secure-frontend) - BFF pattern
- Mobile Apps: Direct API access to auth-service
- Databases: MongoDB (primary), Redis (caching/blacklist)
- Observability: Prometheus, Loki, Grafana, Tempo, Promtail

## Architecture Principles

### Security-First Design

**Zero Trust & Defense in Depth:**
- All service clients must register and authenticate
- Request signing (HMAC-SHA256) for BFF communication
- Service clients bypass bot detection and rate limiting via signature headers
- Mobile clients subject to full security controls (rate limiting, bot detection)
- JWT tokens: RS256 with rotation, revocation, and Redis-backed blacklisting

**Client Types:**
1. **Service Clients** (e.g., secure-frontend): Trusted server-to-server with `rate_limit_per_min: 0`
2. **Mobile Clients**: Subject to IP-based rate limiting and bot detection
3. **Service Accounts**: Scoped API keys for internal microservices

### Layered Architecture (auth-service)

```
Transport Layer (Handlers)
    ↓
Middleware Layer (Security, Rate Limiting, Observability)
    ↓
Service Layer (Business Logic: JwtService, EmailService, AuthService)
    ↓
Data Access Layer (MongoDB Models, Redis)
```

**Key Concept:** `AppState` struct contains all shared dependencies (DB, Redis, JWT service, rate limiters) and is injected via Axum's `State` extractor.

### Middleware Execution Order (auth-service)

Middleware runs **outside-in** (reverse order of application):

1. CORS
2. Bot detection (skips if `X-Signature` header present)
3. Security headers (HSTS, CSP, etc.)
4. Request ID generation
5. Tracing/logging
6. Metrics collection
7. **Global IP rate limiting** (all requests)
8. **Signature validation** (BFF requests)
9. Route-specific rate limiting (login, register, password reset, app token)
10. Route handlers

**Critical:** Service clients with signed requests bypass bot detection (line 37-39 in `auth-service/src/middleware/bot_detection.rs`).

## Development Commands

### Environment Setup

**First-time setup (auth-service):**
```bash
cd auth-service
cp .env.example .env
mkdir -p keys
openssl genrsa -out keys/private.pem 2048
openssl rsa -in keys/private.pem -pubout -out keys/public.pem
```

**Required environment files:**
- Root `.env`: PLG stack ports, service credentials
- `auth-service/.env`: Service config, MongoDB/Redis URLs, JWT settings, OAuth credentials
- `secure-frontend/.env`: Auth service URL, client credentials

### Building

```bash
# Build auth-service
cd auth-service
cargo build --release

# Build secure-frontend
cd secure-frontend
cargo build --release

# Build with Docker (from root)
docker-compose build
```

### Running Services

**Development (individual services):**
```bash
# Auth service (requires MongoDB + Redis)
cd auth-service
cargo run

# Secure frontend (requires auth-service)
cd secure-frontend
cargo run
```

**Full stack with observability:**
```bash
# From root directory
docker-compose up -d

# View logs
docker-compose logs -f auth-service
docker-compose logs -f secure-frontend

# Stop all
docker-compose down
```

**Default ports (host → container):**
- auth-service: 9096 → 3000
- secure-frontend: 9097 → 8080
- Grafana: 9092 → 3000 (admin/admin)
- Prometheus: 9090 → 9090
- Loki: 9091 → 3100
- Tempo: 3200 → 3200
- MongoDB: 9094 → 27017
- Redis: 9095 → 6379

### Testing

```bash
# Run all tests (auth-service)
cd auth-service
cargo test

# Run specific test
cargo test login_test

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

**Test organization:** All integration tests in `auth-service/tests/` (e.g., `login_test.rs`, `security_controls_test.rs`, `signature_middleware_test.rs`).

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check

# Lint with Clippy
cargo clippy

# Clippy with warnings as errors (CI)
cargo clippy -- -D warnings
```

**Pre-commit hook:** `scripts/pre-commit-frontend.sh` runs `cargo fmt --check` and `cargo clippy` on secure-frontend.

### API Documentation

```bash
# Start auth-service
cd auth-service && cargo run

# Access Swagger UI
open http://localhost:9096/docs

# OpenAPI spec
curl http://localhost:9096/.well-known/openapi.json
```

**Auto-generated from code:** Uses `utoipa` macros on handlers. All endpoints documented in `src/lib.rs` `#[derive(OpenApi)]` section.

## Key Implementation Details

### Request Signing (BFF Pattern)

**Signature generation (HMAC-SHA256):**
```rust
let string_to_sign = format!("{}{}{}{}{}", method, path, timestamp, nonce, body);
let signature = hmac_sha256(signing_secret, string_to_sign);
```

**Required headers:**
- `X-Client-ID`: Client UUID
- `X-Timestamp`: Unix timestamp (60-second validity window)
- `X-Nonce`: UUID v4 (stored in Redis for replay prevention)
- `X-Signature`: Hex-encoded HMAC

**Implementation:** `auth-service/src/middleware/signature.rs` and `auth-service/docs/bff-request-signing.md`

### Rate Limiting

**IP-based limiters (Governor + DashMap):**
- Login: 5 attempts / 15 minutes
- Register: 3 attempts / 1 hour
- Password reset: 3 attempts / 1 hour
- Global IP: 100 requests / 60 seconds

**Client-specific limiter:**
- Set `rate_limit_per_min: 0` for unlimited access (service clients)
- Non-zero values enforce per-client quotas

**Key file:** `auth-service/src/middleware/rate_limit.rs`

### JWT Token Lifecycle

**Token types:**
1. **Access Token:** 15 minutes (configurable via `JWT_ACCESS_TOKEN_EXPIRY_MINUTES`)
2. **Refresh Token:** 7 days (configurable via `JWT_REFRESH_TOKEN_EXPIRY_DAYS`)
3. **App Token:** 60 minutes (service-to-service)

**Revocation:** Tokens stored in Redis blacklist on logout. Introspection endpoint checks blacklist before validating signature.

**Key rotation:** JWT keys in `auth-service/keys/`. Rotate by generating new keys and updating deployment.

### OpenTelemetry Tracing

**Configuration:** Both services export to Tempo via OTLP/gRPC on `tempo:4317`.

**Trace initialization (auth-service):**
- File: `auth-service/src/lib.rs:393-424`
- Service name from config: `auth-service` or `secure-frontend`
- Trace IDs automatically propagated through middleware

**Log-trace correlation:** Structured logs include `trace_id` and `span_id` fields for correlation in Grafana.

## Service Registration (Service Clients)

**Register secure-frontend with auth-service:**

```bash
curl -X POST http://localhost:9096/auth/admin/clients \
  -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "app_name": "secure-frontend",
    "app_type": "service",
    "rate_limit_per_min": 0,
    "allowed_origins": ["http://localhost:9097"]
  }'
```

**Store credentials:**
- `client_id` → `APP_AUTH_SERVICE__CLIENT_ID`
- `signing_secret` → `APP_AUTH_SERVICE__SIGNING_SECRET`
- `client_secret` → For app token requests (not used by BFF pattern)

**Benefits:**
- Unlimited rate limits (bypass all rate limiting)
- Bot detection exemption (requests with `X-Signature` header)
- Secure server-to-server authentication via HMAC signing

## Observability

### Structured Logging

**Format:** JSON to stdout (captured by Promtail)

**Required fields:**
- `level`: debug, info, warn, error, fatal
- `msg`: Human-readable message
- `timestamp`: RFC 3339 or Unix timestamp

**Static labels (indexed in Loki):**
- `service`: auth-service or secure-frontend
- `container`: Docker container name
- `level`: Log level

**Dynamic fields (queried in JSON):**
- `trace_id`, `span_id`: Trace correlation
- `request_id`: Request correlation
- `http_method`, `http_status`, `http_url`: Request details
- `error`: Error messages

### Metrics Collection

**Prometheus scraping:**
- `/metrics` endpoint exposed on both services
- Scrape interval: 15 seconds
- Auto-discovery via Docker labels

**Common metrics:**
- `http_requests_total`: Request counter with labels (method, status, path)
- `http_request_duration_seconds`: Histogram of request latency
- Custom business metrics via `prometheus` crate

### Grafana Dashboards

**Pre-configured:**
- `config/grafana/dashboards/auth_service.json`
- `config/grafana/dashboards/secure_frontend.json`

**Datasources (auto-provisioned):**
- Prometheus (metrics)
- Loki (logs) - with trace-to-logs correlation
- Tempo (traces) - with trace-to-logs and trace-to-metrics correlation

## Skills Reference

Located in `skills/` directory. Use these as reference when implementing features:

- **deployment-automation**: Docker deployment patterns with zero-downtime
- **environment-config**: Dev/prod environment management
- **functional-programming**: Rust functional patterns
- **logging-design**: PLG-compatible structured logging
- **rest-api-development**: RESTful API design patterns
- **rest-api-security**: Rate limiting, bot detection, authentication
- **rust-backend-processes**: Graceful shutdown, retry logic, progress tracking
- **rust-development**: Security, thread safety, functional patterns
- **rust-htmx-frontend**: Htmx patterns with Askama templates
- **service-observability**: PLG stack configuration

## Common Gotchas

### Docker Volume Permissions

Tempo requires proper volume permissions. Configuration in `docker-compose.yml` uses `user: "10001:10001"` and volume mount at `/var/tempo` (not `/tmp/tempo`).

### JWT Key Generation

Auth-service will not start without JWT keys. Always generate via OpenSSL before first run:
```bash
cd auth-service/keys
openssl genrsa -out private.pem 2048
openssl rsa -in private.pem -pubout -out public.pem
```

### Middleware Order Matters

Rate limiting must come **after** signature validation in middleware stack. Otherwise, legitimate service clients get rate limited before signature check exempts them.

Current (correct) order in `auth-service/src/lib.rs:278-312`:
1. Global IP rate limiting
2. Signature validation
3. Route-specific middleware (including route-level rate limiting)

### Request Signing Timestamp Validation

Timestamp must be within 60 seconds of server time. Clock skew between services can cause signature validation failures. Use NTP to sync clocks in production.

### Loki Label Cardinality

Do **not** use high-cardinality fields (request_id, user_id, trace_id) as Loki labels. These should be JSON fields queryable with LogQL. Only use static labels: `service`, `container`, `level`, `env`.

## Configuration Files

**Critical paths:**
- `auth-service/.env`: Service configuration
- `auth-service/keys/`: JWT RS256 key pair
- `config/prometheus/prometheus.yml`: Scrape targets
- `config/loki/loki.yaml`: Log ingestion
- `config/promtail/promtail.yaml`: Log shipping with JSON parsing
- `config/tempo/tempo.yaml`: Trace ingestion
- `config/grafana/provisioning/datasources/datasource.yml`: Grafana datasources
- `docker-compose.yml`: Full stack orchestration

## Documentation

**Auth Service docs (auth-service/docs/):**
- `email-password-auth.md`: Registration and login flows
- `security-controls.md`: Rate limiting and bot detection
- `bff-request-signing.md`: HMAC signature implementation
- `service-integration.md`: Service account setup
- `observability.md`: Logging and tracing configuration

**Project docs (docs/):**
- `configuration-summary-2026-01-08.md`: Complete setup and architecture reference

## External Dependencies

**Production dependencies:**
- MongoDB 6.0+
- Redis 7+
- SMTP server (Gmail for dev, transactional email service for prod)

**Optional (development):**
- Google OAuth credentials (for social login)
- Admin API key (set via `ADMIN_API_KEY` env var)
