# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Micros** is a production-ready Rust microservices monorepo with gRPC communication and full PLG (Prometheus, Loki, Grafana) + Tempo observability stack.

**Services (9 microservices + shared library):**
- `auth-service`: Authentication, authorization, capability-based access control (PostgreSQL)
- `billing-service`: Billing and subscription management (PostgreSQL)
- `document-service`: Document storage and retrieval (MongoDB)
- `genai-service`: Generative AI with Gemini integration (MongoDB)
- `invoicing-service`: Invoice generation and management (PostgreSQL)
- `ledger-service`: Financial ledger and double-entry accounting (PostgreSQL)
- `notification-service`: Multi-channel notifications - email, SMS, push (MongoDB)
- `payment-service`: Payment processing with Razorpay integration (MongoDB)
- `reconciliation-service`: Payment reconciliation and dispute handling (PostgreSQL)
- `service-core`: Shared middleware, gRPC utilities, observability, error handling
- `workflow-tests`: Cross-service integration tests

**Communication:** gRPC (Tonic) for inter-service communication; HTTP only for health checks and metrics.

**Databases:** PostgreSQL (auth, billing, invoicing, ledger, reconciliation) and MongoDB (document, genai, notification, payment).

## Development Commands

```bash
# Build
cargo build                      # All services
cargo build -p auth-service      # Specific service

# Test
cargo test                       # All tests
cargo test -p payment-service    # Specific service
cargo test login_test            # Specific test
cargo test -- --nocapture        # With output
./scripts/integ-tests.sh         # Integration tests (requires DBs)
./scripts/integ-tests.sh -p workflow-tests  # Cross-service tests (requires dev stack)

# Code quality
cargo fmt                        # Format
cargo clippy                     # Lint
cargo clippy -- -D warnings      # Strict mode (CI)

# Docker
./scripts/dev-up.sh              # Start dev stack
./scripts/dev-down.sh            # Stop dev stack
docker-compose -f docker-compose.dev.yml logs -f auth-service  # View logs
```

**First-time setup:**
```bash
cp .env.example .env.dev
mkdir -p auth-service/keys
openssl genrsa -out auth-service/keys/private.pem 2048
openssl rsa -in auth-service/keys/private.pem -pubout -out auth-service/keys/public.pem

# Start prerequisites on host:
# - PostgreSQL (5432), MongoDB (27017), Redis (6379)
# - PLG+T observability stack:
cd observability && ./start.sh
```

## Architecture

### gRPC-First Design

All service-to-service communication uses gRPC (Tonic). Protocol Buffer definitions in `proto/micros/`:
- `auth/v1/`: Authentication, roles, org hierarchy, capabilities
- `payment/v1/`, `ledger/v1/`, `billing/v1/`, `invoicing/v1/`: Financial services
- `notification/v1/`, `document/v1/`, `genai/v1/`, `reconciliation/v1/`
- `common/`: Shared error types, pagination, metadata

**Service Ports:**
| Service | Health (HTTP) | gRPC |
|---------|---------------|------|
| auth | 9005 | 50051 |
| document | 9007 | 50052 |
| notification | 9008 | 50053 |
| payment | 9009 | 50054 |
| genai | 9010 | 50055 |
| ledger | 9011 | 50056 |
| billing | 9012 | 50057 |
| reconciliation | 9013 | 50058 |
| invoicing | 9014 | 50059 |

### Layered Architecture

Each service follows:
```
Transport Layer (gRPC handlers in src/grpc/)
    ↓
Middleware Layer (Tonic interceptors from service-core)
    ↓
Service Layer (Business logic in src/services/)
    ↓
Data Access Layer (src/db/ - PostgreSQL via sqlx or MongoDB)
```

**Standard service structure:**
```
service-name/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Public exports
│   ├── startup.rs        # Application lifecycle (testable pattern)
│   ├── config/           # Configuration loading
│   ├── db/               # Database setup, migrations
│   ├── grpc/             # gRPC service implementations
│   ├── handlers/         # HTTP handlers (health, metrics)
│   ├── models/           # Domain models, DTOs
│   └── services/         # Business logic
├── migrations/           # SQL migrations (PostgreSQL services)
├── tests/                # Integration tests
│   └── common/mod.rs     # TestApp helper
└── Dockerfile
```

### service-core

Shared infrastructure used by all services:

- **grpc/**: Client wrappers, interceptors (tracing, metrics, retry), capability checker, health checks
- **middleware/**: HTTP middleware (signature validation, rate limiting, bot detection, security headers)
- **observability/**: Structured JSON logging, OpenTelemetry tracing to Tempo
- **error.rs**: Unified `AppError` type with Axum/Tonic response conversion

### TestApp Pattern

Integration tests spawn isolated servers on random ports:
```rust
pub struct TestApp {
    pub http_address: String,
    pub grpc_address: String,
    pub db: Database,
}

impl TestApp {
    pub async fn spawn() -> Self { /* creates isolated test instance */ }
    pub async fn cleanup(&self) { /* drops test schema */ }
}
```

PostgreSQL tests use unique schemas (`test_inv_<pid>_<counter>`) for isolation.

## Key Implementation Details

### Database Configuration

**PostgreSQL services** use sqlx with migrations in `*/migrations/` directories.

**MongoDB services** connect via `MONGODB_URI` environment variable.

Tests require databases running locally:
- PostgreSQL: `localhost:5432`
- MongoDB: `localhost:27017`
- Redis: `localhost:6379`

### Docker Build

Uses centralized builder pattern (`Dockerfile.builder`) to compile all services in one pass, avoiding memory exhaustion from parallel builds.

```bash
docker build -f Dockerfile.builder -t micros-builder .
docker compose build
```

### Environment Configuration

Single `.env` file per environment (`.env.dev`, `.env.prod`). No service-specific env files.

- **Dev**: Databases on host via `host.docker.internal`, services on ports 9005-9014
- **Prod**: Everything containerized, ports 10005-10014

### Observability

PLG+T (Prometheus, Loki, Grafana, Tempo) runs on the host machine as a prerequisite for `dev-up.sh`.

**Standard Ports:**
| Service | Port | Purpose |
|---------|------|---------|
| Prometheus | 9090 | Metrics scraping & storage |
| Loki | 3100 | Log aggregation |
| Grafana | 3000 | Dashboards UI |
| Tempo | 3200 | Trace query API |
| Tempo OTLP | 4317 | Trace ingestion (gRPC) |

**Commands:**
```bash
cd observability && ./start.sh   # Start PLG+T stack (required before dev-up.sh)
cd observability && ./stop.sh    # Stop PLG+T stack
```

**Data Flow:**
- **Traces**: Services → OTLP (`host.docker.internal:4317`) → Tempo
- **Logs**: Services → stdout (JSON) → Promtail → Loki (`host.docker.internal:3100`)
- **Metrics**: Prometheus scrapes `/health` endpoints

**App-Specific Logging:**
Services emit only app-specific logs (no framework noise). Configured via:
```
RUST_LOG=<service>=${LOG_LEVEL},service_core=${LOG_LEVEL}
```

See `docs/specs/observability.md` for full specification.

## Common Gotchas

**Workspace dependencies**: Use `{ workspace = true }` in service Cargo.toml to inherit versions from root.

**service-core changes**: Trigger full rebuild of all services. Use `cargo build -p <service>` during development.

**gRPC ports**: Services expose both HTTP (health/metrics) and gRPC ports. Check docker-compose for mappings.

**Test isolation**: PostgreSQL tests use `--test-threads=1` to prevent race conditions on shared databases.

**JWT keys required**: auth-service won't start without keys. Generate via OpenSSL (see first-time setup).

**Proto changes**: After modifying `.proto` files, services auto-regenerate on build via `build.rs`.
