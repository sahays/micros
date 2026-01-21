# Micros - Microservices Monorepo

Production-ready Rust microservices with gRPC interfaces and full observability stack.

## Architecture

```
┌─────────────────┐     REST      ┌──────────────────┐
│  Browsers       │◄────────────► │  secure-frontend │
│  Mobile Apps    │               │  (BFF)           │
└─────────────────┘               └────────┬─────────┘
                                           │ gRPC
              ┌────────────────────────────┼────────────────────────────┐
              │                            ▼                            │
              │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
              │  │ auth-service │  │ document-svc │  │notification  │  │
              │  │   :50051     │  │   :50052     │  │   :50053     │  │
              │  └──────────────┘  └──────────────┘  └──────────────┘  │
              │                                                         │
              │  ┌──────────────┐  ┌──────────────┐                     │
              │  │payment-svc   │  │ genai-svc    │  Internal Network   │
              │  │   :50054     │  │   :50055     │  (gRPC only)        │
              │  └──────────────┘  └──────────────┘                     │
              └─────────────────────────────────────────────────────────┘
```

## Services

| Service | Port | Description | README |
|---------|------|-------------|--------|
| auth-service | 50051 | Authentication, authorization, multi-tenant | [README](./auth-service/README.md) |
| document-service | 50052 | Document storage with streaming | [README](./document-service/README.md) |
| notification-service | 50053 | Email, SMS, push notifications | [README](./notification-service/README.md) |
| payment-service | 50054 | Razorpay, UPI payments | [README](./payment-service/README.md) |
| genai-service | 50055 | Generative AI with Gemini | [README](./genai-service/README.md) |
| secure-frontend | 8080 | BFF with REST API for clients | - |
| service-core | - | Shared middleware and utilities | - |

## Quick Start

```bash
# 1. Copy environment template
cp .env.example .env.dev

# 2. Start MongoDB and Redis locally

# 3. Start all services
./scripts/dev-up.sh

# Access points (dev):
# - Grafana: http://localhost:9002 (admin/admin)
# - Auth gRPC: localhost:50051
# - Document gRPC: localhost:50052
# - Notification gRPC: localhost:50053
# - Payment gRPC: localhost:50054
# - GenAI gRPC: localhost:50055
```

## gRPC Usage

```bash
# Install grpcurl
brew install grpcurl

# List services (uses reflection)
grpcurl -plaintext localhost:50051 list

# Call an RPC
grpcurl -plaintext -d '{"tenant_slug":"acme","email":"user@example.com","password":"secret"}' \
  localhost:50051 micros.auth.v1.AuthService/Login
```

## Development

```bash
cargo build              # Build all
cargo test               # Test all
cargo fmt                # Format
cargo clippy             # Lint
cargo run -p auth-service  # Run specific service
```

## Observability

- **Prometheus**: Metrics (port 9000/10000)
- **Loki**: Logs (port 9001/10001)
- **Grafana**: Dashboards (port 9002/10002)
- **Tempo**: Traces (port 9003/10003)

## Documentation

- [CLAUDE.md](./CLAUDE.md) - Complete architecture guide
- [docs/plan/](./docs/plan/) - Epic and task planning
- [proto/](./proto/) - Protocol buffer definitions
