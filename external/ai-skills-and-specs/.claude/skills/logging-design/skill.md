---
name: logging-design
description:
  Design effective application logging from a software engineering perspective. Use when implementing logging in code,
  choosing log levels, or defining what to log. Focuses on structured logging, security, and developer best practices.
---

# Logging Design for Engineers

## Structured Logging

Use structured formats (JSON) with consistent fields. Makes logs machine-readable and searchable.

**Every log entry must include**:

- `timestamp`: ISO 8601 with timezone
- `level`: Severity level
- `message`: Human-readable description
- `service`: Service name
- `request_id`: Correlation ID

**Add context as separate fields, not in message string**:

- `user_id`, `session_id`, `tenant_id`
- `operation`, `duration_ms`
- `error_code`, `stack_trace`

**Good**: `log.info("User login", {user_id: "123", method: "oauth"})`

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

**Generate request ID at entry point**: Propagate through entire request lifecycle. Include in all logs.

**Propagate context**: Pass request_id, user_id, trace_id through function calls and async operations.

**Log at boundaries**: Service entry/exit, external API calls, database operations.

## Performance

**Log asynchronously**: Never block application threads on logging.

**Use appropriate levels**: INFO in production. DEBUG only during troubleshooting.

**Lazy evaluation**: Defer expensive operations until confirmed log level matches.

**Circuit breaker**: Fail gracefully if logging unavailable. Don't crash the application.

## Error Logging

**Include full context**:

- Exception type and message
- Complete stack trace
- Request details that triggered error
- User context (sanitized)
- What was attempted and why it failed

**Use error codes**: Unique identifiers for error categories. Makes searching and monitoring easier.

**Log once per error**: Catch and log at appropriate level. Don't re-log as error bubbles up.

## Security

**Sanitize inputs**: Remove sensitive data before logging. Use allow-list approach.

**Hash PII**: If you need to correlate by user_id or session_id, hash them.

**No secrets in errors**: Stack traces and error messages shouldn't expose credentials or keys.

## Message Design

**Be specific**: "User authentication failed" not "Error in login"

**Use past tense**: "Payment processed" not "Processing payment"

**Add context in fields**: Don't interpolate values into message string

**Action-oriented**: Focus on what happened, not code implementation details
