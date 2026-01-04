---
name: logging-design
description:
  Design effective application logging from a software engineering perspective. Use when implementing logging in code,
  choosing log levels, or defining what to log. Focuses on structured logging with OpenTelemetry standards for SigNoz.
---

# Logging Design for Engineers

## Structured Logging

Use structured formats (JSON) with consistent fields. Makes logs machine-readable and searchable. Follow OpenTelemetry semantic conventions for compatibility with SigNoz and other observability platforms.

**Every log entry must include**:

- `timestamp`: Unix nanoseconds or ISO 8601 with timezone (OpenTelemetry standard)
- `severity_text`: Severity level (use: TRACE, DEBUG, INFO, WARN, ERROR, FATAL)
- `severity_number`: Numeric severity (1=TRACE, 5=DEBUG, 9=INFO, 13=WARN, 17=ERROR, 21=FATAL)
- `body`: Human-readable log message
- `resource.service.name`: Service identifier
- `trace_id`: 16-byte trace ID (32 hex chars) for distributed tracing
- `span_id`: 8-byte span ID (16 hex chars) for current operation

**Add context as separate attributes, not in body string**:

- `user.id`, `session.id`, `enduser.id`
- `http.method`, `http.status_code`, `http.route`, `http.target`
- `db.system`, `db.operation`, `db.statement`
- `rpc.service`, `rpc.method`
- `exception.type`, `exception.message`, `exception.stacktrace`
- `code.function`, `code.namespace`, `code.filepath`, `code.lineno`

**Good**: `log.info("User login", {attributes: {"user.id": "123", "http.method": "POST", "enduser.id": "user-123"}})`

**Bad**: `log.info("User 123 login via oauth")`

## Log Levels

**TRACE**: Step-by-step execution. Development only. High volume.

**DEBUG**: Detailed diagnostics. Development and troubleshooting only.

**INFO**: Normal operations. Default for production. Application lifecycle, business events.

**WARN**: Unexpected but handled. Degraded functionality, fallbacks, retries.

**ERROR**: Failures that don't crash the service. Caught exceptions, failed operations.

**FATAL**: Unrecoverable errors causing shutdown.

**Production default**: INFO. DEBUG creates performance problems and excessive volume.

## What to Log

**Application lifecycle**: Start/stop, version, configuration changes

**Request boundaries**: HTTP requests (method, path, status, duration), API calls to external services

**Business events**: State transitions (order created, payment processed, user registered)

**Authentication events**: Login success/failure, authorization failures, token operations

**Errors**: Exception type, message, stack trace, request context, user impact

**Performance**: Operations exceeding thresholds, slow queries, timeouts

## What NOT to Log

**Never log**:

- Passwords, API keys, tokens, secrets
- Credit card numbers, SSNs
- Full email addresses or phone numbers (hash if needed)
- Session tokens
- Sensitive business data

**Sanitize before logging**: Request/response bodies, query parameters, headers

## Context and Correlation

**Generate trace ID at entry point**: Propagate through entire request lifecycle. Include in all logs as `trace_id`. Use OpenTelemetry SDK to generate W3C Trace Context compliant IDs.

**Propagate context**: Pass `trace_id`, `span_id`, `trace_flags` through function calls and async operations using OpenTelemetry Context API.

**Log at boundaries**: Service entry/exit, external API calls, database operations. Use semantic convention attributes like `http.method`, `db.operation`, `rpc.method` to identify operations.

## Performance

**Log asynchronously**: Never block application threads on logging.

**Use appropriate levels**: INFO in production. DEBUG only during troubleshooting.

**Lazy evaluation**: Defer expensive operations until confirmed log level matches.

**Circuit breaker**: Fail gracefully if logging unavailable. Don't crash the application.

## Error Logging

**Include full context**:

- `exception.type`: Exception class name
- `exception.message`: Exception message
- `exception.stacktrace`: Complete stack trace
- `http.*`: Request details that triggered error using OpenTelemetry HTTP semantic conventions
- `user.id` or `enduser.id`: User context (sanitized)
- `error.code`: Application-specific error code

**Use error codes**: Set `error.code` attribute with unique identifiers for error categories. Makes searching and monitoring easier.

**Log once per error**: Catch and log at appropriate level. Don't re-log as error bubbles up.

**Link to traces**: Ensure `trace_id` and `span_id` are present so logs correlate with distributed traces in SigNoz.

## Security

**Sanitize inputs**: Remove sensitive data before logging. Use allow-list approach.

**Hash PII**: If you need to correlate by user_id or session_id, hash them.

**No secrets in errors**: Stack traces and error messages shouldn't expose credentials or keys.

## Message Design

**Be specific**: "User authentication failed" not "Error in login"

**Use past tense**: "Payment processed" not "Processing payment"

**Add context in fields**: Don't interpolate values into message string

**Action-oriented**: Focus on what happened, not code implementation details
