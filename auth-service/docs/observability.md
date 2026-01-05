# Observability & Google Cloud Integration

The Auth Service is designed with "Observability by Default". It outputs structured JSON logs and traces compatible with Google Cloud Logging and Cloud Trace.

## 1. Structured Logging

We use `tracing` with a JSON formatter. All logs are emitted to `stdout`.

**Format:**
```json
{
  "timestamp": "2024-01-05T12:00:00Z",
  "level": "INFO",
  "fields": {
    "message": "User logged in",
    "user_id": "550e8400-...",
    "request_id": "d290f1ee-...",
    "service": "auth-service",
    "environment": "prod"
  },
  "target": "auth_service::handlers::auth"
}
```

### Google Cloud Logging Integration

When deployed to Google Cloud (Cloud Run, GKE, or GCE):

1.  **Automatic Ingestion:** The Cloud Logging agent (fluentd-based) automatically captures `stdout`.
2.  **JSON Parsing:** GCP detects the JSON format and parses fields into `jsonPayload`.
3.  **Searching:** You can query logs in the Logs Explorer using structured filters:
    ```
    jsonPayload.fields.user_id="550e8400..." AND severity="INFO"
    ```

## 2. Request Tracing

### X-Request-ID
Every request is assigned a unique `X-Request-ID`.
-   If the header is present in the incoming request, it is preserved.
-   If missing, a new UUID is generated.
-   It is included in the response headers.
-   It is attached to every log entry within that request's scope.

**Scenario: Debugging a Failure**
1.  Client reports an error with `X-Request-ID: abc-123`.
2.  Go to Google Cloud Logging.
3.  Query: `jsonPayload.fields.request_id="abc-123"`.
4.  You will see all logs (Info, Warn, Error) associated with that single request flow.

## 3. Health Checks

**Endpoint:** `GET /health`

Returns the status of the service and its dependencies (MongoDB, Redis).

**Response:**
```json
{
  "status": "healthy",
  "service": "auth-service",
  "checks": {
    "mongodb": "up",
    "redis": "up"
  }
}
```

### Google Cloud Monitoring (Stackdriver)

Configure a **Uptime Check** in Google Cloud Monitoring pointing to `/health`.
-   **Frequency:** Every 1-5 minutes.
-   **Alerting:** Trigger an alert if status is not `200 OK` or response JSON `status` != `healthy`.

## 4. Performance Metrics

While current implementation focuses on logs, the `tracing` spans include duration data.

**Querying Latency in Logs:**
You can approximate latency metrics by analyzing the `http_request` span logs which record `time.busy` or duration.

For dedicated metrics, future integration with `prometheus` or `opentelemetry` exporters would be required.
