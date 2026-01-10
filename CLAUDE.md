# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Micros** is a production-ready microservices monorepo implementing secure, observable Rust-based services with full PLG (Prometheus, Loki, Grafana) + Tempo observability stack.

**Stack:**
- Backend Services: Rust/Axum microservices
  - `auth-service`: Authentication and authorization
  - `document-service`: Document storage and retrieval with S3/local storage
  - `service-core`: Shared middleware, observability, and utilities
- Web Frontend: Rust/Axum/Htmx (secure-frontend) - BFF pattern
- Mobile Apps: Direct API access to microservices
- Databases: MongoDB (primary), Redis (caching/blacklist)
- Observability: Prometheus, Loki, Grafana, Tempo, Promtail

**Workspace Structure:**
- All services share dependencies via Cargo workspace (`Cargo.toml` in root)
- `service-core` provides reusable infrastructure: middleware (signature validation, rate limiting, bot detection), observability (tracing, logging), error handling, and configuration utilities

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

### Layered Architecture

All services follow a consistent layered architecture:

```
Transport Layer (Handlers)
    ↓
Middleware Layer (Security, Rate Limiting, Observability)
    ↓
Service Layer (Business Logic)
    ↓
Data Access Layer (MongoDB, Redis, S3)
```

**Key Concepts:**
- Each service has an `AppState` struct containing shared dependencies (DB clients, service instances, configuration)
- State is injected into handlers via Axum's `State` extractor
- `service-core` provides reusable middleware, observability utilities, and error types
- Services use `Application` pattern for testable server lifecycle management (see `startup.rs` files)

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

### Environment Configuration Philosophy

**Single Source of Truth:**
- All configuration consolidated in ONE root `.env` file (`.env.dev` or `.env.prod`)
- No service-specific `.env` files - eliminates confusion and duplication
- Services read from environment variables passed by Docker Compose

**Development vs Production:**
- **Development (`.env.dev` + `docker-compose.dev.yml`):**
  - MongoDB and Redis run on **host machine** at standard ports (27017, 6379)
  - Services connect via `host.docker.internal`
  - Services exposed on ports 9000-9009
  - Docker project name: `micros-dev`
  - Containers prefixed: `micros-dev-auth-service`, `micros-dev-grafana`, etc.

- **Production (`.env.prod` + `docker-compose.prod.yml`):**
  - **Everything** containerized including MongoDB and Redis
  - Services exposed on ports 10000-10009
  - Docker project name: `micros-prod`
  - Containers prefixed: `micros-prod-auth-service`, `micros-prod-grafana`, etc.
  - Full isolation and portability

**Why this approach?**
- **No duplication**: One file to edit, no sync issues
- **Clear separation**: Explicit dev vs prod configurations
- **Complete isolation**: Separate Docker networks, volumes, and containers
- **Run simultaneously**: Dev and prod can run at the same time without conflicts
- **Flexible**: Easy to add new services or change configuration
- **Portable**: Copy `.env.example` to get started

## Development Commands

### Environment Setup

**First-time setup:**
```bash
# 1. Copy environment template
cp .env.example .env.dev

# 2. Generate JWT keys for auth-service
mkdir -p auth-service/keys
openssl genrsa -out auth-service/keys/private.pem 2048
openssl rsa -in auth-service/keys/private.pem -pubout -out auth-service/keys/public.pem

# 3. Start MongoDB and Redis on your host machine
# (Dev: Services connect to host MongoDB/Redis)
# (Prod: MongoDB/Redis run in Docker Compose)

# 4. Edit .env.dev and set secrets:
#    - APP_SERVER__SESSION_SECRET
#    - APP_AUTH_SERVICE__SIGNING_SECRET
#    - ADMIN_API_KEY
#    - GRAFANA_ADMIN_PASSWORD
```

**Environment Configuration:**
- **Single source of truth**: All configuration in ONE root `.env` file
- **`.env.example`**: Template showing all available configuration options
- **`.env.dev`**: Development configuration (MongoDB/Redis on host, non-standard service ports)
- **`.env.prod`**: Production configuration (everything containerized, non-standard ports)
- **No service-specific `.env` files**: All services read from environment variables passed by Docker Compose

### Building

```bash
# Build all workspace members (from root)
cargo build

# Build specific service
cargo build -p auth-service
cargo build -p document-service
cargo build -p secure-frontend
cargo build -p service-core

# Build release version
cd auth-service
cargo build --release

# Build with Docker (from root)
docker-compose build

# Build specific service with Docker
docker-compose build auth-service
docker-compose build document-service
```

### Running Services

**Development (Docker Compose):**
```bash
# Start services
./scripts/dev-up.sh

# View logs
docker-compose -f docker-compose.dev.yml logs -f auth-service
docker-compose -f docker-compose.dev.yml logs -f document-service

# Stop services
./scripts/dev-down.sh

# Manual way (if not using scripts):
docker-compose -f docker-compose.dev.yml --env-file .env.dev up -d
docker-compose -f docker-compose.dev.yml down

# Rebuild and restart specific service
docker-compose -f docker-compose.dev.yml up -d --build auth-service
```

**Production (Docker Compose):**
```bash
# Start services
./scripts/prod-up.sh

# View logs
docker-compose -f docker-compose.prod.yml logs -f

# Stop services
./scripts/prod-down.sh

# Manual way (if not using scripts):
docker-compose -f docker-compose.prod.yml --env-file .env.prod up -d
docker-compose -f docker-compose.prod.yml down

# Stop and remove volumes (deletes all data)
docker-compose -f docker-compose.prod.yml down -v
```

**Port Configuration:**

*Development (9000-9009 range):*
- Prometheus: **9000** → 9090
- Loki: **9001** → 3100
- Grafana: **9002** → 3000 (admin/admin)
- Tempo: **9003** → 3200
- Promtail: **9004** → 9080
- auth-service: **9005** → 3000
- secure-frontend: **9006** → 8080
- document-service: **9007** → 8080
- MongoDB: **27017** (running on host machine, NOT containerized)
- Redis: **6379** (running on host machine, NOT containerized)

*Production (10000-10009 range):*
- Prometheus: **10000** → 9090
- Loki: **10001** → 3100
- Grafana: **10002** → 3000
- Tempo: **10003** → 3200
- Promtail: **10004** → 9080
- auth-service: **10005** → 3000
- secure-frontend: **10006** → 8080
- document-service: **10007** → 8080
- MongoDB: **10008** → 27017 (containerized)
- Redis: **10009** → 6379 (containerized)

**Both environments can run simultaneously without port conflicts.**

### Complete Environment Isolation

**Docker Desktop Project Separation:**
- Dev environment appears as **`micros-dev`** in Docker Desktop
- Prod environment appears as **`micros-prod`** in Docker Desktop
- Separate containers, networks, and volumes for each environment

**Isolation Details:**
- **Containers**: `micros-dev-auth-service` vs `micros-prod-auth-service`
- **Networks**: `micros-dev_network` vs `micros-prod_network`
- **Volumes**: `micros-dev_prometheus_data` vs `micros-prod_prometheus_data`

**Benefits:**
- Start/stop one environment without affecting the other
- Different data in each environment (separate databases)
- Compare dev vs prod behavior side-by-side
- Clean separation visible in Docker Desktop UI

### Testing

```bash
# Run all tests in workspace (from root)
cargo test

# Run tests for specific service
cargo test -p auth-service
cargo test -p document-service
cargo test -p service-core

# Run specific test
cd auth-service
cargo test login_test

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run tests with limited parallelism (reduces memory usage)
cargo test --jobs 2
```

**Test organization:**
- Integration tests in `*/tests/` directories (e.g., `auth-service/tests/login_test.rs`, `document-service/tests/health_check.rs`)
- Unit tests alongside implementation code in `src/` files
- Tests use the `Application` pattern to spawn test servers on random ports

### Code Quality

```bash
# Format all workspace code
cargo fmt

# Format specific service
cd auth-service && cargo fmt

# Check formatting (CI)
cargo fmt -- --check

# Lint with Clippy (all workspace)
cargo clippy

# Clippy for specific service
cargo clippy -p document-service

# Clippy with warnings as errors (CI)
cargo clippy -- -D warnings
```

**Pre-commit hooks:**
- `scripts/pre-commit.sh`: Main hook that runs fmt, clippy, and tests on all changed services (auth-service, document-service, service-core, secure-frontend)
- `scripts/pre-commit-frontend.sh`: Frontend-specific checks
- Install: `ln -s ../../scripts/pre-commit.sh .git/hooks/pre-commit`

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

### service-core Architecture

The `service-core` library provides shared infrastructure used across all microservices:

**Middleware (`service-core/src/middleware/`):**
- `signature.rs`: HMAC-SHA256 request signature validation for BFF pattern
- `rate_limit.rs`: IP-based and client-based rate limiting using Governor + DashMap
- `bot_detection.rs`: User-Agent based bot detection (bypassed for signed requests)
- `security_headers.rs`: Security headers (HSTS, CSP, X-Frame-Options, etc.)
- `metrics.rs`: Prometheus metrics collection middleware
- `tracing.rs`: OpenTelemetry tracing middleware

**Observability (`service-core/src/observability/`):**
- `logging.rs`: Structured JSON logging configuration for PLG stack
- Automatic trace ID and span ID injection into logs

**Utilities (`service-core/src/utils/`):**
- Signature generation and validation helpers
- Configuration loading utilities

**Error Handling (`service-core/src/error.rs`):**
- Unified `AppError` type with automatic Axum response conversion
- Error variants: DatabaseError, ConfigError, ValidationError, etc.

**When adding new services:** Import `service-core` as a dependency and reuse middleware instead of reimplementing security/observability logic.

### document-service Architecture

**Storage Abstraction (`document-service/src/services/`):**
- `Storage` trait defines interface for file operations (upload, download, delete)
- `LocalStorage`: Filesystem-based storage for development
- `S3Storage`: AWS S3 storage for production
- Configured via `STORAGE_BACKEND` environment variable (local/s3)

**Database Layer:**
- MongoDB for document metadata (owner_id, filename, content_type, size, timestamps)
- Automatic index creation on `owner_id` for fast lookups
- Health check via MongoDB ping command

**File Upload Flow:**
1. Multipart form data received at `POST /documents`
2. File saved to storage backend (local/S3)
3. Metadata persisted to MongoDB with storage path
4. Returns document ID and metadata

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
- `service`: auth-service, document-service, or secure-frontend
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

**Pre-configured dashboards:**
- `config/grafana/dashboards/auth_service.json`
- `config/grafana/dashboards/secure_frontend.json`

**Datasources (auto-provisioned):**
- Prometheus (metrics) - scrapes all services with `prometheus.io/scrape=true` label
- Loki (logs) - with trace-to-logs correlation
- Tempo (traces) - with trace-to-logs and trace-to-metrics correlation

**Adding dashboards for new services:** Create JSON dashboard in `config/grafana/dashboards/` and add Prometheus labels to service in `docker-compose.yml`.

## Adding New Services to the Workspace

Follow this pattern when adding a new microservice:

1. **Create service directory structure:**
   ```bash
   mkdir new-service
   cd new-service
   cargo init --lib
   mkdir -p src/{handlers,models,services,config} tests
   ```

2. **Add to workspace** (`Cargo.toml` in root):
   ```toml
   [workspace]
   members = ["auth-service", "document-service", "new-service", "service-core"]
   ```

3. **Configure dependencies** (`new-service/Cargo.toml`):
   ```toml
   [dependencies]
   service-core = { path = "../service-core" }
   axum = { workspace = true }
   mongodb = { workspace = true }
   # Use workspace dependencies where possible
   ```

4. **Implement standard structure:**
   - `src/lib.rs`: Module exports
   - `src/main.rs`: Entry point
   - `src/startup.rs`: `Application` struct with testable server lifecycle
   - `src/config/mod.rs`: Service configuration using `config` crate
   - `src/handlers/`: Axum handlers
   - `src/services/`: Business logic
   - `src/models/`: Data models
   - `tests/`: Integration tests

5. **Add Docker configuration:**
   - Create `new-service/Dockerfile` (use auth-service as template)
   - Add service to `docker-compose.yml` with health checks
   - Add Prometheus scrape labels

6. **Configure observability:**
   - Initialize tracing to Tempo (see `service-core/src/observability/logging.rs`)
   - Add `/health` endpoint
   - Add `/metrics` endpoint
   - Create Grafana dashboard in `config/grafana/dashboards/`

7. **Update pre-commit hook** (`scripts/pre-commit.sh`):
   Add new service to the list of checked services

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

### Cargo Workspace Dependencies

When adding dependencies to a service, check if it's already defined in the root `Cargo.toml` `[workspace.dependencies]` section. Use `{ workspace = true }` to inherit workspace versions:

```toml
# In service-specific Cargo.toml
[dependencies]
axum = { workspace = true }  # Correct - inherits from workspace
tokio = "1.28"               # Wrong - creates version conflict
```

This ensures all services use the same dependency versions and speeds up compilation.

### service-core Changes Trigger Full Rebuild

Since all services depend on `service-core`, any change to service-core will trigger recompilation of all services. Use `cargo build -p <service>` to build only the service you're working on during development.

### Docker Volume Permissions

Tempo requires proper volume permissions. Configuration in `docker-compose.yml` uses `user: "10001:10001"` and volume mount at `/var/tempo` (not `/tmp/tempo`).

### JWT Key Generation

Auth-service will not start without JWT keys. Always generate via OpenSSL before first run:
```bash
cd auth-service/keys
openssl genrsa -out private.pem 2048
openssl rsa -in keys/private.pem -pubout -out keys/public.pem
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

### Storage Backend Configuration (document-service)

When using S3 storage backend, ensure AWS credentials are configured via environment variables or IAM roles. Local storage creates a `storage/` directory in the service root - ensure it's in `.gitignore`.

## Configuration Files

**Environment Configuration:**
- `.env.example`: Template with all configuration variables (committed to git)
- `.env.dev`: Development configuration (NOT in git, create from template)
- `.env.prod`: Production configuration (NOT in git, create from template)
- **No service-specific `.env` files**: All configuration centralized in root

**Critical paths:**
- `Cargo.toml`: Workspace configuration and shared dependencies
- `.env.dev` / `.env.prod`: All service configuration in one place
- `docker-compose.dev.yml`: Development stack (no MongoDB/Redis containers)
- `docker-compose.prod.yml`: Production stack (includes MongoDB/Redis)
- `auth-service/keys/`: JWT RS256 key pair (generate via OpenSSL)
- `config/prometheus/prometheus.yml`: Scrape targets
- `config/loki/loki.yaml`: Log ingestion
- `config/promtail/promtail.yaml`: Log shipping with JSON parsing
- `config/tempo/tempo.yaml`: Trace ingestion and retention
- `config/grafana/provisioning/datasources/datasource.yml`: Grafana datasources
- `config/grafana/dashboards/`: Pre-built service dashboards

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
