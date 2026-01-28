# Observability Specification

## Overview

The micros platform uses the PLG+T stack (Prometheus, Loki, Grafana, Tempo) for full observability. The stack runs on the host machine separately from the application services, following the same pattern as PostgreSQL, MongoDB, and Redis.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Host Machine                                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              PLG+T Stack (observability-dev)                 │   │
│  │  ┌───────────┐ ┌──────┐ ┌─────────┐ ┌───────┐               │   │
│  │  │ Prometheus│ │ Loki │ │ Grafana │ │ Tempo │               │   │
│  │  │   :9090   │ │:3100 │ │  :3000  │ │ :3200 │               │   │
│  │  └───────────┘ └──────┘ └─────────┘ │ :4317 │               │   │
│  │                    ▲                 └───────┘               │   │
│  └────────────────────│─────────────────────▲───────────────────┘   │
│                       │                     │                        │
│  ┌────────────────────│─────────────────────│───────────────────┐   │
│  │           Micros Services (micros-dev)   │                   │   │
│  │  ┌──────────┐                            │                   │   │
│  │  │ Promtail │────────────────────────────┘                   │   │
│  │  └──────────┘         (logs via host.docker.internal:3100)   │   │
│  │       ▲                                                      │   │
│  │       │ (container logs)                                     │   │
│  │  ┌────┴─────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │   │
│  │  │   auth   │ │ billing │ │ ledger  │ │  ...    │           │   │
│  │  │ service  │ │ service │ │ service │ │         │           │   │
│  │  └──────────┘ └─────────┘ └─────────┘ └─────────┘           │   │
│  │       │            │           │           │                 │   │
│  │       └────────────┴───────────┴───────────┘                 │   │
│  │                         │                                    │   │
│  │              (traces via host.docker.internal:4317)          │   │
│  └─────────────────────────│────────────────────────────────────┘   │
│                            ▼                                         │
│                      Tempo (OTLP)                                    │
└─────────────────────────────────────────────────────────────────────┘
```

## Components

### Prometheus (Port 9090)

Metrics collection and storage.

**Configuration:** `observability/prometheus/prometheus.yml`

**Scrape Targets:**
- All micros services via `host.docker.internal:<health_port>/health`
- Loki (`loki:3100`)
- Tempo (`tempo:3200`)

### Loki (Port 3100)

Log aggregation and querying.

**Configuration:** `observability/loki/loki.yaml`

**Storage:** Local filesystem at `/loki/chunks`

**Retention:** Configured via schema_config (24h index period)

### Grafana (Port 3000)

Visualization and dashboards.

**Credentials:** admin/admin (default)

**Provisioned Datasources:**
- Prometheus (`http://prometheus:9090`)
- Loki (`http://loki:3100`)
- Tempo (`http://tempo:3200`)

**Dashboards:** Pre-built in `observability/grafana/dashboards/`

### Tempo (Ports 3200, 4317, 4318)

Distributed tracing backend.

**Configuration:** `observability/tempo/tempo.yaml`

**Endpoints:**
| Port | Protocol | Purpose |
|------|----------|---------|
| 3200 | HTTP | Query API |
| 4317 | gRPC | OTLP ingestion |
| 4318 | HTTP | OTLP ingestion (alt) |
| 14268 | HTTP | Jaeger ingest |
| 9411 | HTTP | Zipkin |

### Promtail

Log shipper running inside the micros docker-compose (not in observability stack).

**Configuration:** `config/promtail/promtail.yaml`

**Push Target:** `http://host.docker.internal:3100/loki/api/v1/push`

**Scrape Source:** Docker container logs via `/var/run/docker.sock`

## Directory Structure

```
observability/
├── docker-compose.yml          # PLG+T stack definition
├── start.sh                    # Start script (sets COMPOSE_PROJECT_NAME)
├── stop.sh                     # Stop script
├── prometheus/
│   └── prometheus.yml          # Scrape configuration
├── loki/
│   └── loki.yaml               # Loki configuration
├── tempo/
│   └── tempo.yaml              # Tempo configuration
└── grafana/
    ├── provisioning/
    │   ├── datasources/
    │   │   └── datasource.yml  # Auto-provisioned datasources
    │   └── dashboards/
    │       └── dashboard.yml   # Dashboard provisioning config
    └── dashboards/
        └── *.json              # Pre-built dashboards
```

## Logging Configuration

### App-Specific Logging

Services are configured to emit only application-specific logs, filtering out framework noise from tonic, hyper, sqlx, mongodb, tokio, etc.

**RUST_LOG Configuration:**
```yaml
RUST_LOG=${RUST_LOG:-<service_crate>=${LOG_LEVEL:-info},service_core=${LOG_LEVEL:-info}}
```

**Example for auth-service:**
```yaml
RUST_LOG=auth_service=info,service_core=info
```

This filters logs to only show:
- `auth_service::*` - Application code
- `service_core::*` - Shared infrastructure

**Filtered out:**
- `tonic::*` - gRPC framework
- `hyper::*` - HTTP framework
- `h2::*` - HTTP/2 protocol
- `tokio::*` - Async runtime
- `sqlx::*` - Database driver
- `mongodb::*` - Database driver
- `tower::*` - Middleware framework

### Log Format

All services emit structured JSON logs:

```json
{
  "timestamp": "2026-01-28T07:49:30.804960Z",
  "level": "INFO",
  "message": "Starting auth-service v2 (gRPC)",
  "target": "auth_service",
  "filename": "auth-service/src/main.rs",
  "line_number": 38,
  "service": "auth-service",
  "version": "0.1.0",
  "environment": "Dev"
}
```

### Controlling Log Levels

```bash
# Set via environment variable
LOG_LEVEL=debug ./scripts/dev-up.sh

# Or override completely
RUST_LOG=debug ./scripts/dev-up.sh
```

## Tracing Configuration

### OpenTelemetry Setup

Services use OpenTelemetry with OTLP exporter configured in `service-core/src/observability/logging.rs`:

```rust
pub fn init_tracing(service_name: &str, log_level: &str, otlp_endpoint: &str) {
    // Configures:
    // 1. Environment filter for log levels
    // 2. OTLP exporter to Tempo
    // 3. JSON formatter for structured logs
}
```

### OTLP Endpoint

Services connect to Tempo via:
```
OTLP_ENDPOINT=http://host.docker.internal:4317
```

### Trace Context Propagation

Trace context is propagated between services using W3C Trace Context headers:
- `traceparent`
- `tracestate`
- `x-request-id`

## Usage

### Starting the Stack

```bash
# Start PLG+T (required before starting services)
cd observability && ./start.sh

# For production environment
cd observability && ./start.sh --prod
```

### Stopping the Stack

```bash
cd observability && ./stop.sh

# Remove volumes (deletes all data)
cd observability && COMPOSE_PROJECT_NAME=observability-dev docker-compose down -v
```

### Health Checks

```bash
curl http://localhost:9090/-/healthy  # Prometheus
curl http://localhost:3100/ready      # Loki
curl http://localhost:3200/ready      # Tempo
curl http://localhost:3000/api/health # Grafana
```

### Querying Logs (Loki)

```bash
# List available labels
curl -s "http://localhost:3100/loki/api/v1/labels"

# Query logs from auth-service
curl -sG "http://localhost:3100/loki/api/v1/query_range" \
  --data-urlencode 'query={container="micros-dev-auth-service"}' \
  --data-urlencode 'limit=10'
```

### Querying Traces (Tempo)

```bash
# Search for traces
curl -s "http://localhost:3200/api/search?limit=10"

# Search by service name
curl -s "http://localhost:3200/api/search?tags=service.name%3Dauth-service&limit=5"
```

## Prerequisites Check

The `scripts/dev-up.sh` script verifies PLG+T is running before starting services:

```bash
Checking PLG+T observability stack...
✓ Prometheus is accessible on port 9090
✓ Loki is accessible on port 3100
✓ Grafana is accessible on port 3000
✓ Tempo OTLP is accessible on port 4317
```

If any component is missing, the script fails with:
```
PLG+T observability stack is not fully running
Please start it first: cd observability && ./start.sh
```

## Docker Network Configuration

### Observability Stack Network
- Network: `observability_network`
- Containers: `observability-dev-{prometheus,loki,grafana,tempo}`

### Micros Services Network
- Network: `micros-dev_network`
- Services connect to PLG+T via `host.docker.internal`

### Port Mappings

| Service | Container Port | Host Port |
|---------|----------------|-----------|
| Prometheus | 9090 | 9090 |
| Loki | 3100 | 3100 |
| Grafana | 3000 | 3000 |
| Tempo API | 3200 | 3200 |
| Tempo OTLP gRPC | 4317 | 4317 |
| Tempo OTLP HTTP | 4318 | 4318 |

## Volume Management

Observability data is persisted in Docker volumes:
- `observability-dev_prometheus_data`
- `observability-dev_loki_data`
- `observability-dev_tempo_data`
- `observability-dev_grafana_data`

To clear all observability data:
```bash
cd observability && COMPOSE_PROJECT_NAME=observability-dev docker-compose down -v
```
