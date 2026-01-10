---
name: logging-design
description:
  Design effective application logging from a software engineering perspective. Use when implementing logging in code,
  choosing log levels, or defining what to log. Focuses on structured logging for PLG stack (Prometheus, Loki, Grafana).
---

- Structured Logging (JSON)
  - Use JSON format for all logs
  - Required: level (debug, info, warn, error, fatal)
  - Required: msg (human-readable message)
  - Required: ts (Unix timestamp or RFC 3339 format)
  - Static labels (low cardinality, extracted by Promtail): service, environment, host, version
  - Dynamic fields (high cardinality, stored in JSON): request_id, user_id, duration_ms, http_*
  - http_method, http_url, http_status, http_user_agent, http_latency_ms for request logs
  - trace_id: distributed trace identifier for correlation
  - span_id: span identifier within trace
  - caller: source location as "file:line" or "package.function"
  - error: error message or type
  - stack: stack trace string for errors

- Log Levels
  - debug: verbose information for debugging, high volume
  - info: routine operational events, normal system behavior
  - warn: unexpected events that don't prevent operation
  - error: errors that affect specific operations but system continues
  - fatal: severe errors requiring immediate shutdown
  - Production default: info

- Loki Label Strategy
  - Static labels (indexed, use for filtering): service, env, host, level, job
  - Keep label cardinality low (< 10 values per label)
  - DO NOT use request_id, user_id, trace_id as labels (query JSON fields instead)
  - Promtail extracts static labels from JSON or uses pipeline stages
  - Example labels: {service="api", env="prod", host="web01", level="error"}

- What to Log
  - Application lifecycle: startup parameters, version, config hash, shutdown reason
  - Request boundaries: ingress/egress with http_* fields, duration, status
  - Business logic: state transitions, transaction outcomes, key operations
  - Errors: error messages, stack traces, context fields
  - Security: authentication failures, authorization denials (no tokens/passwords)

- What NOT to Log
  - Secrets: passwords, API keys, tokens, session IDs, PII
  - High cardinality data as labels (use JSON fields instead)
  - Redundant errors (log once at error boundary, not at every stack level)
  - Noisy debug logs in production (use dynamic log level control)

- Error Tracking
  - Log at error or fatal level
  - Include error field with error type or message
  - Include stack field with full stack trace
  - Add context fields: request_id, user_id, endpoint, input parameters
  - Use Grafana alerting rules to detect error rate spikes
  - Optional: service and version fields for multi-service deployments

- Context and Correlation
  - Propagate trace_id across service boundaries (HTTP header: X-Trace-Id)
  - Include trace_id and span_id in all logs within request context
  - Use LogQL queries in Grafana to filter by trace_id for request tracing
  - Service identity via static labels: service, host, environment

- Best Practices
  - Write JSON logs to stdout/stderr for Promtail collection
  - Use asynchronous/buffered logging to avoid blocking application threads
  - Implement structured logging library (zap, zerolog, slog, winston, bunyan)
  - Add request_id to all logs in request lifecycle for correlation
  - Use log sampling for high-volume debug logs (sample 1% of requests)
  - Consistent timestamp format across all services (RFC 3339 or Unix ms)
  - Configure Promtail to parse JSON and extract static labels
  - Rotate log files if writing to disk (size or time-based rotation)