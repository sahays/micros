# Observability & PLG Stack

The Auth Service is designed with "Observability by Default". It uses a self-hosted PLG stack (Prometheus, Loki, Grafana) for metrics and logs.

## 1. Structured Logging

We use `tracing` with a JSON formatter. All logs are emitted to `stdout` and collected by Promtail. Events are flattened to the root of the JSON object for easier parsing.

**Format:**
```json
{
  "timestamp": "2024-01-05T12:00:00Z",
  "level": "INFO",
  "message": "User logged in",
  "user_id": "550e8400-...",
  "request_id": "d290f1ee-...",
  "service": "auth-service",
  "environment": "prod",
  "target": "auth_service::handlers::auth",
  "file": "src/handlers/auth.rs",
  "line": 42
}
```

### PLG Stack Integration

The stack is defined in `docker-compose.yml` and consists of:

1.  **Promtail:** Scrapes Docker container logs (stdout/stderr) via the Docker socket. It parses the JSON logs and extracts labels like `level`, `container`, etc.
2.  **Loki:** Stores the log streams.
3.  **Grafana:** Visualizes logs using the Loki datasource.

**Querying Logs in Grafana:**
Use LogQL in the Explore view:
```
{container="auth-service"} | json | level="INFO"
```

## 2. Request Tracing

### X-Request-ID
Every request is assigned a unique `X-Request-ID`.
-   If the header is present, it is preserved.
-   If missing, a new UUID is generated.
-   It is included in the response headers.
-   It is attached to the main `http_request` span and propagated to all log entries.

**Scenario: Debugging a Failure**
1.  Client reports an error with `X-Request-ID: abc-123`.
2.  Go to Grafana -> Explore -> Loki.
3.  Query: `{container="auth-service"} | json | request_id="abc-123"`.

## 3. Metrics (Prometheus)

The service exposes a `/metrics` endpoint compatible with Prometheus.

**Exposed Metrics:**
-   `http_requests_total{method, path, status}`: Counter of total requests.
-   `http_request_duration_seconds{method, path, status}`: Histogram of request latency.

### Prometheus Integration
Prometheus is configured to automatically discover the service via Docker labels (`prometheus.io/scrape=true`).

**Querying Metrics in Grafana:**
-   Rate of requests: `rate(http_requests_total[5m])`
-   99th percentile latency: `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))`

## 4. Health Checks

**Endpoint:** `GET /health`

Returns the status of the service and dependencies (MongoDB, Redis). The Docker container uses this endpoint for its internal health check.

## 5. Local Development

To start the observability stack alongside the service:
```bash
docker-compose up -d
```
Access Grafana at `http://localhost:3000` (User: `admin`, Password: `admin`).