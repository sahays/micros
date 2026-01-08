# Micros Stack - Configuration Summary

**Version:** 2026-01-08
**Author:** Claude (Sonnet 4.5)
**Status:** Complete

## Overview

This document summarizes the configuration changes made to the micros monorepo to establish a production-ready Rust+Htmx microservices architecture with full observability.

## Architecture

**Stack:**
- **Backend API:** Rust/Axum (auth-service)
- **Web Frontend:** Rust/Axum/Htmx (secure-frontend)
- **Mobile Apps:** Direct API access to auth-service
- **Observability:** PLG Stack (Prometheus, Loki, Grafana) + Tempo

**Client Types:**
- **Service Clients:** Trusted server-to-server (e.g., secure-frontend) - unlimited rate limits, bypass bot detection
- **Mobile Clients:** Direct API calls - subject to rate limiting and security controls

## Changes Completed

### 1. Removed sample-frontend (React/Next.js)

**Rationale:** Standardizing on Rust+Htmx for web development.

**Changes:**
- Deleted `/sample-frontend` directory
- Updated `/scripts/pre-commit-frontend.sh` to check `secure-frontend` instead
  - Runs `cargo fmt --check` for Rust code formatting
  - Runs `cargo clippy` for Rust linting

**Files Modified:**
- `scripts/pre-commit-frontend.sh`

**Files Deleted:**
- `sample-frontend/` (entire directory)

---

### 2. Service Client Exemptions (auth-service)

**Requirement:** Exempt trusted service clients (BFF) from rate limiting and bot detection while maintaining security controls for mobile apps.

**Implementation:**

#### Bot Detection Bypass
**File:** `auth-service/src/middleware/bot_detection.rs`

Added exemption for requests with signature headers (service-to-service authentication):

```rust
// Skip service-to-service calls (identified by signature headers)
// Service clients use request signing for authentication
if headers.contains_key("X-Signature") || headers.contains_key("x-signature") {
    return Ok(next.run(request).await);
}
```

**Line:** 35-39

#### Rate Limiting Exemption
**File:** `auth-service/src/middleware/rate_limit.rs`

Existing implementation already supports unlimited access:

```rust
// Skip if limit is 0 (unlimited)
if limit_per_min == 0 {
    return next.run(request).await;
}
```

**Line:** 171-173

**Configuration:** Set `rate_limit_per_min: 0` when creating service clients.

#### Documentation Updates
**File:** `auth-service/docs/security-controls.md`

Updated sections:
- Rate limiting strategy (line 25-26)
- Bot detection exemptions (lines 38-64)

---

### 3. PLG Stack Configuration Fixes

**Issues Resolved:**

#### Missing JWT Keys
**Problem:** auth-service failed to start (missing `keys/private.pem` and `keys/public.pem`)

**Solution:**
```bash
cd auth-service/keys
openssl genrsa -out private.pem 2048
openssl rsa -in private.pem -pubout -out public.pem
```

**Files Created:**
- `auth-service/keys/private.pem`
- `auth-service/keys/public.pem`

#### Tempo Configuration Schema Errors
**Problem:** Tempo container crash-looping with config parsing errors

**File:** `config/tempo/tempo.yaml`

**Changes:**
- Removed deprecated `ingester.max_chunk_bytes` field
- Removed deprecated `compactor.max_block_bytes` field
- Simplified configuration to match Tempo v2.9.0 schema
- Changed storage paths from `/tmp/tempo` to `/var/tempo`

**Updated Configuration:**
```yaml
server:
  http_listen_port: 3200

distributor:
  receivers:
    otlp:
      protocols:
        grpc:
          endpoint: 0.0.0.0:4317
        http:
          endpoint: 0.0.0.0:4318

storage:
  trace:
    backend: local
    local:
      path: /var/tempo
    wal:
      path: /var/tempo/wal

overrides:
  defaults:
    metrics_generator:
      processors: [service-graphs, span-metrics]
```

#### Docker Compose Improvements
**File:** `docker-compose.yml`

**Changes:**
- Removed obsolete `version: "3.8"` field (line 1)
- Updated Tempo volume mount from `/tmp/tempo` to `/var/tempo`
- Added `user: "10001:10001"` for Tempo (proper permissions)
- Added `TEMPO_PORT` environment variable support (defaults to 3200)

---

## Service Configuration

### Port Mapping (Host → Container)

| Service | Host Port | Container Port | Internal URL | External URL |
|---------|-----------|----------------|--------------|--------------|
| **Grafana** | 9092 | 3000 | http://grafana:3000 | http://localhost:9092 |
| **Prometheus** | 9090 | 9090 | http://prometheus:9090 | http://localhost:9090 |
| **Loki** | 9091 | 3100 | http://loki:3100 | http://localhost:9091 |
| **Promtail** | 9093 | 9080 | http://promtail:9080 | http://localhost:9093 |
| **Tempo** | 3200 | 3200 | http://tempo:3200 | http://localhost:3200 |
| **auth-service** | 9096 | 3000 | http://auth-service:3000 | http://localhost:9096 |
| **secure-frontend** | 9097 | 8080 | http://secure-frontend:8080 | http://localhost:9097 |
| **MongoDB** | 9094 | 27017 | mongodb://mongo:27017 | mongodb://localhost:9094 |
| **Redis** | 9095 | 6379 | redis://redis:6379 | redis://localhost:9095 |

**Note:** Services use internal URLs for inter-service communication within the `micros-net` Docker network.

### Environment Variables

All services support customizable ports via environment variables in `.env`:

```bash
# Observability
GRAFANA_PORT=9092
PROMETHEUS_PORT=9090
LOKI_PORT=9091
PROMTAIL_PORT=9093
TEMPO_PORT=3200

# Application Services
AUTH_SERVICE_PORT=9096
SECURE_FRONTEND_PORT=9097

# Data Stores
MONGO_PORT=9094
REDIS_PORT=9095
```

---

## Observability Stack Details

### Prometheus (Metrics)

**Configuration:** `config/prometheus/prometheus.yml`

**Scrape Targets:**
- `auth-service:3000/metrics` (15s interval)
- `secure-frontend:8080/metrics` (15s interval)
- `prometheus:9090/metrics` (self-monitoring)
- `loki:3100/metrics`
- `promtail:9080/metrics`
- Docker containers (via service discovery)

**Verification:**
```bash
curl "http://localhost:9090/api/v1/targets"
```

**Status:** ✅ All targets healthy and scraping

---

### Loki (Logs)

**Configuration:** `config/loki/loki.yaml`

**Features:**
- JSON log parsing
- Label extraction (service, level, container)
- 15-day retention

**Verification:**
```bash
curl "http://localhost:9091/loki/api/v1/label/container/values"
```

**Status:** ✅ Collecting logs from 9 containers:
- auth-service
- secure-frontend
- grafana
- loki
- mongo
- prometheus
- promtail
- redis
- tempo

---

### Promtail (Log Shipper)

**Configuration:** `config/promtail/promtail.yaml`

**Features:**
- Docker service discovery
- JSON log parsing for auth-service and secure-frontend
- Automatic label extraction (container, service, level)
- Docker socket scraping (`/var/run/docker.sock`)

**Pipeline Stages:**
```yaml
- docker: {}
- match:
    selector: '{container=~"auth-service|secure-frontend"}'
    stages:
      - json:
          expressions:
            level: level
      - template:
          source: service
          template: '{{ .container }}'
      - labels:
          level:
          service:
```

---

### Tempo (Distributed Tracing)

**Configuration:** `config/tempo/tempo.yaml`

**Features:**
- OTLP gRPC ingestion (port 4317)
- OTLP HTTP ingestion (port 4318)
- Service graph generation
- Span metrics generation
- Correlation with Loki logs
- Correlation with Prometheus metrics

**Trace Exporters Configured:**
- auth-service: OpenTelemetry → Tempo (gRPC endpoint: `http://tempo:4317`)
- secure-frontend: OpenTelemetry → Tempo (gRPC endpoint: `http://tempo:4317`)

**Verification:**
```bash
curl "http://localhost:3200/ready"
```

**Status:** ✅ Running (metrics generator warnings are normal during initial startup)

---

### Grafana (Visualization)

**Configuration:** `config/grafana/provisioning/datasources/datasource.yml`

**Pre-configured Datasources:**
1. **Prometheus** (default)
   - URL: http://prometheus:9090
   - Type: Metrics

2. **Loki**
   - URL: http://loki:3100
   - Type: Logs
   - Trace-to-logs correlation enabled

3. **Tempo**
   - URL: http://tempo:3200
   - Type: Traces
   - Trace-to-logs correlation (→ Loki)
   - Trace-to-metrics correlation (→ Prometheus)
   - Service graph enabled

**Pre-configured Dashboards:**
- `config/grafana/dashboards/auth_service.json`
- `config/grafana/dashboards/secure_frontend.json`

**Access:**
- URL: http://localhost:9092
- Default credentials: `admin` / `admin`

**Status:** ✅ All datasources connected and healthy

---

## Service Client Registration

To register `secure-frontend` (or any service client) for unrestricted API access:

### Step 1: Create Client via Admin API

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

**Response:**
```json
{
  "client_id": "uuid-v4-here",
  "client_secret": "plaintext-secret-here",
  "signing_secret": "hmac-secret-here",
  "app_name": "secure-frontend",
  "app_type": "service",
  "rate_limit_per_min": 0,
  "allowed_origins": ["http://localhost:9097"]
}
```

**⚠️ Important:** Store credentials securely. `client_secret` and `signing_secret` are shown only once.

---

### Step 2: Configure secure-frontend Environment

Update `.env` or `docker-compose.yml`:

```bash
APP_AUTH_SERVICE__URL=http://auth-service:3000
APP_AUTH_SERVICE__CLIENT_ID=<client_id from response>
APP_AUTH_SERVICE__SIGNING_SECRET=<signing_secret from response>
```

**File:** `secure-frontend/.env` or `docker-compose.yml` environment section

---

### Step 3: Implement Request Signing

**Reference:** `auth-service/docs/bff-request-signing.md`

**Required Headers:**
```
X-Client-ID: <client_id>
X-Timestamp: <unix_timestamp>
X-Nonce: <unique_random_value>
X-Signature: <hmac_sha256_signature>
```

**Signature Format:**
```
HMAC-SHA256(signing_secret, METHOD + PATH + TIMESTAMP + NONCE + BODY)
```

**Example (pseudocode):**
```rust
let string_to_sign = format!(
    "{}{}{}{}{}",
    method,      // "POST"
    path,        // "/auth/login"
    timestamp,   // "1704672000"
    nonce,       // "random-uuid"
    body         // JSON request body
);

let signature = hmac_sha256(signing_secret, string_to_sign);
let signature_hex = hex::encode(signature);
```

---

### Benefits

With service client registration, `secure-frontend` gains:

✅ **Unlimited Rate Limits**
- Bypasses all rate limiting middleware
- No `429 Too Many Requests` errors

✅ **Bot Detection Exemption**
- Requests with `X-Signature` header skip bot detection
- No false positives from missing browser headers

✅ **Secure Authentication**
- HMAC request signing prevents tampering
- Nonce prevents replay attacks
- Timestamp limits request validity window (60 seconds)

✅ **Trusted Communication**
- Server-to-server authentication
- Prevents malicious clients from impersonating secure-frontend
- Audit trail for all service-to-service calls

---

## Verification & Testing

### Health Checks

**All Services:**
```bash
docker-compose ps
```

Expected: All containers show `Up` or `Up (healthy)` status.

**Auth Service:**
```bash
curl http://localhost:9096/health
```

Expected:
```json
{
  "status": "healthy",
  "service": "auth-service",
  "version": "0.1.0",
  "environment": "Dev",
  "checks": {
    "mongodb": "up",
    "redis": "up"
  }
}
```

**Secure Frontend:**
```bash
curl http://localhost:9097/health
```

---

### Metrics Collection

**Prometheus Targets:**
```bash
curl -s "http://localhost:9090/api/v1/targets" | jq '.data.activeTargets[] | select(.scrapePool | contains("auth-service") or contains("secure-frontend")) | {job: .scrapePool, health: .health}'
```

Expected:
```json
{
  "job": "auth-service",
  "health": "up"
}
{
  "job": "secure-frontend",
  "health": "up"
}
```

**Service Metrics Query:**
```bash
curl -s "http://localhost:9090/api/v1/query?query=up{job=~\"auth-service|secure-frontend\"}" | jq '.data.result[] | {job: .metric.job, up: .value[1]}'
```

Expected:
```json
{
  "job": "auth-service",
  "up": "1"
}
{
  "job": "secure-frontend",
  "up": "1"
}
```

---

### Log Collection

**Available Containers in Loki:**
```bash
curl -s "http://localhost:9091/loki/api/v1/label/container/values" | jq '.data[]'
```

Expected output includes:
```
"auth-service"
"secure-frontend"
```

**Query Service Logs:**
```bash
# Auth service logs (last 10 lines)
curl -s "http://localhost:9091/loki/api/v1/query_range?query=%7Bcontainer%3D%22auth-service%22%7D&limit=10" | jq '.data.result[0].values[] | .[1]'

# Secure frontend logs (last 10 lines)
curl -s "http://localhost:9091/loki/api/v1/query_range?query=%7Bcontainer%3D%22secure-frontend%22%7D&limit=10" | jq '.data.result[0].values[] | .[1]'
```

---

### Trace Collection

**Tempo Status:**
```bash
curl http://localhost:3200/ready
```

Note: May show "Ingester not ready" for 15s after startup. This is normal.

**Service Instrumentation:**

Both services export traces via OpenTelemetry:
- Protocol: OTLP/gRPC
- Endpoint: `http://tempo:4317`
- Configuration: `auth-service/src/lib.rs:397-409`

Traces will appear in Grafana's Tempo datasource after the first requests are made to the services.

---

## Grafana Usage

### Access Grafana

1. Open http://localhost:9092
2. Login with `admin` / `admin`
3. (Optional) Change password when prompted

### Explore Logs

1. Click **Explore** (compass icon) in left sidebar
2. Select **Loki** datasource
3. Query examples:
   - All auth-service logs: `{container="auth-service"}`
   - Error logs only: `{container="auth-service"} | json | level="ERROR"`
   - Logs with trace correlation: `{container="auth-service"} | json | trace_id!=""`

### Explore Metrics

1. Click **Explore**
2. Select **Prometheus** datasource
3. Query examples:
   - Request rate: `rate(http_requests_total{job="auth-service"}[5m])`
   - Error rate: `rate(http_requests_total{job="auth-service",status=~"5.."}[5m])`
   - Latency P95: `histogram_quantile(0.95, http_request_duration_seconds_bucket{job="auth-service"})`

### Explore Traces

1. Click **Explore**
2. Select **Tempo** datasource
3. Search by:
   - Service name: `auth-service` or `secure-frontend`
   - Trace ID (from logs)
   - Time range

### View Dashboards

1. Click **Dashboards** (squares icon)
2. Pre-configured dashboards:
   - Auth Service Dashboard
   - Secure Frontend Dashboard

---

## Known Issues & Workarounds

### Tempo Metrics Generator Warnings

**Issue:** Tempo logs show periodic warnings:
```
failed to forward request to metrics generator: DoBatch: InstancesCount <= 0
```

**Impact:** None. Metrics generator warnings are normal during startup and low traffic.

**Workaround:** Ignore. Will resolve once traces are flowing.

---

### Promtail Container Inspection Errors

**Issue:** Promtail logs show errors:
```
could not inspect container info: No such container: <old_container_id>
```

**Impact:** None. Occurs when containers are recreated.

**Workaround:** Ignore. Promtail continues to collect logs from active containers.

---

## File Changes Summary

### Modified Files

| File | Changes |
|------|---------|
| `scripts/pre-commit-frontend.sh` | Updated to check `secure-frontend` (Rust) instead of `sample-frontend` (React/Next.js) |
| `auth-service/src/middleware/bot_detection.rs` | Added service client exemption (lines 35-39) |
| `auth-service/docs/security-controls.md` | Updated bot detection and rate limiting documentation |
| `config/tempo/tempo.yaml` | Fixed schema for Tempo v2.9.0 compatibility |
| `docker-compose.yml` | Removed `version` field, updated Tempo configuration |

### Created Files

| File | Purpose |
|------|---------|
| `auth-service/keys/private.pem` | JWT signing private key (RS256) |
| `auth-service/keys/public.pem` | JWT verification public key (RS256) |
| `docs/configuration-summary-2026-01-08.md` | This document |

### Deleted Files

| Path | Reason |
|------|--------|
| `sample-frontend/` | Removed React/Next.js frontend (using Rust+Htmx instead) |

---

## References

### Auth Service Documentation

- Email/Password Auth: `auth-service/docs/email-password-auth.md`
- Security Controls: `auth-service/docs/security-controls.md`
- BFF Request Signing: `auth-service/docs/bff-request-signing.md`
- Service Integration: `auth-service/docs/service-integration.md`
- Social Login: `auth-service/docs/social-login.md`
- Audit Logging: `auth-service/docs/audit-logging.md`
- Observability & PLG: `auth-service/docs/observability.md`
- Bot Detection: `auth-service/docs/bot-detection.md`

### Skills

Located in `skills/`:
- `deployment-automation` - Docker deployment scripts
- `environment-config` - Dev/prod configuration
- `functional-programming` - Functional patterns
- `git-pre-commit` - Pre-commit hooks
- `logging-design` - Structured logging for PLG
- `react-development` - React best practices
- `rest-api-development` - RESTful API design
- `rest-api-security` - API security controls
- `rust-backend-processes` - Rust service patterns
- `rust-development` - Rust best practices
- `rust-htmx-frontend` - Rust+Htmx patterns
- `service-observability` - PLG stack setup
- `spec-driven-development` - Epic/story workflow
- `web-design` - UI/UX design patterns

---

## Next Steps

### Immediate Actions

1. **Register secure-frontend** as a service client
   - Follow "Service Client Registration" section above
   - Store credentials in `.env` file (gitignored)

2. **Verify observability**
   - Access Grafana at http://localhost:9092
   - Check metrics dashboards
   - Verify logs are flowing

3. **Test request signing**
   - Implement HMAC signing in secure-frontend
   - Verify requests pass signature validation
   - Confirm bot detection bypass

### Recommended Enhancements

1. **Production Readiness**
   - Enable HTTPS/TLS
   - Use secrets manager (Vault, AWS Secrets Manager)
   - Configure Grafana authentication (SSO/LDAP)
   - Set up alerting rules

2. **Monitoring**
   - Create custom Grafana dashboards
   - Set up alerts for error rates
   - Configure uptime monitoring

3. **Security**
   - Rotate JWT keys
   - Implement API key rotation
   - Enable audit log review
   - Configure CAPTCHA for high-risk endpoints

4. **Performance**
   - Configure Redis caching
   - Optimize database indexes
   - Implement connection pooling

---

## Changelog

**2026-01-08 - Initial Configuration**
- Removed sample-frontend (React/Next.js)
- Added service client exemptions to auth-service
- Fixed PLG stack configuration issues
- Documented complete architecture and setup
- All services running and healthy

---

## Support & Troubleshooting

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f auth-service
docker-compose logs -f secure-frontend
docker-compose logs -f grafana
```

### Restart Services

```bash
# All services
docker-compose restart

# Specific service
docker-compose restart auth-service
docker-compose restart tempo
```

### Rebuild After Code Changes

```bash
# Rebuild specific service
docker-compose build auth-service
docker-compose up -d auth-service

# Rebuild all services
docker-compose build
docker-compose up -d
```

### Reset Everything

```bash
# Stop and remove containers + volumes
docker-compose down -v

# Restart fresh
docker-compose up -d
```

---

**Document Version:** 1.0
**Last Updated:** 2026-01-08
**Maintained By:** Development Team
