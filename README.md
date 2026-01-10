# Micros - Microservices Monorepo

Production-ready Rust microservices with full observability stack (Prometheus, Loki, Grafana, Tempo).

## Quick Start

### Development Setup

```bash
# 1. Copy environment template
cp .env.example .env.dev

# 2. Start MongoDB and Redis on your host machine
# (Services will connect to host:27017 and host:6379)

# 3. Start all services
./scripts/dev-up.sh

# Services will be available at (Dev: 9000-9009):
# - Auth Service: http://localhost:9005
# - Secure Frontend: http://localhost:9006
# - Document Service: http://localhost:9007
# - Grafana: http://localhost:9002 (admin/admin)
```

### Production Setup

```bash
# 1. Copy environment template
cp .env.example .env.prod

# 2. Edit .env.prod and set all secrets
# (MongoDB and Redis will run in containers)

# 3. Start all services
./scripts/prod-up.sh
```

## Architecture

**Services:**
- `auth-service`: Authentication and authorization (JWT, OAuth, rate limiting)
- `document-service`: Document storage with S3/local backend
- `secure-frontend`: HTMX-based web frontend (BFF pattern)
- `service-core`: Shared middleware and utilities

**Observability:**
- Prometheus (metrics)
- Loki (logs)
- Grafana (dashboards)
- Tempo (traces)

## Environment Configuration

**Single source of truth** - All configuration in one root `.env` file:

- `.env.example`: Template with all options
- `.env.dev`: Development config (MongoDB/Redis on host)
- `.env.prod`: Production config (everything containerized)

**No service-specific `.env` files** - Eliminates duplication and confusion.

**Complete isolation** - Dev and prod environments are completely separate:
- Different Docker project names (`micros-dev` vs `micros-prod`)
- Separate port ranges (9000-9009 vs 10000-10009)
- Independent containers, networks, and volumes
- Can run both environments simultaneously

## Development Commands

```bash
# Build workspace
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Run specific service
cargo run -p auth-service

# Start/stop environments
./scripts/dev-up.sh        # Start dev environment
./scripts/dev-down.sh      # Stop dev environment
./scripts/prod-up.sh       # Start prod environment
./scripts/prod-down.sh     # Stop prod environment

# View logs
docker-compose -f docker-compose.dev.yml logs -f auth-service
docker-compose -f docker-compose.prod.yml logs -f
```

## Documentation

See [CLAUDE.md](./CLAUDE.md) for complete architecture documentation and development guide.

**Service-specific docs:**
- [auth-service/README.md](./auth-service/README.md)
- [auth-service/docs/](./auth-service/docs/) - API guides and security controls

## License

MIT
