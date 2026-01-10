---
name: rust-backend-processes
description:
  Build robust Rust backend processes with retry logic, progress reporting, and graceful shutdown handling. Use when
  implementing long-running services, workers, or background processes requiring resilience and observability.
---

- Graceful Shutdown
  - Signal handling: use tokio::signal to handle SIGTERM, SIGINT
  - Shutdown pattern: receive signal, stop accepting new work, complete in-flight operations (with timeout), clean up resources, exit
  - Tokio shutdown: use CancellationToken or tokio::sync::watch for coordinated shutdown
  - Timeout on shutdown: force shutdown after grace period (30-60 seconds)

- Retry Strategies
  - Exponential backoff: double delay between retries, start at 100ms, cap at 30s
  - Max attempts: limit retry attempts (3-5 for transient errors, unlimited with backoff for critical operations)
  - Jitter: add randomness to backoff to prevent thundering herd
  - Idempotency: ensure operations are safe to retry
  - Library: use backoff crate for retry logic with exponential backoff

- Error Handling
  - Transient vs permanent: retry transient errors (network, timeout), fail fast on permanent (auth, validation)
  - Error classification: categorize as retryable or non-retryable
  - Error context: use anyhow for applications, thiserror for libraries, add context to errors
  - Dead letter queue: move failed items to DLQ after max retries

- Circuit Breaker
  - Purpose: prevent cascading failures when dependency is down
  - States: Closed (normal), Open (failing), Half-Open (testing recovery)
  - Implementation: track failure rate, open circuit after threshold, test with single request in half-open
  - Fast failure: return immediately when circuit is open

- Progress Reporting
  - Persist progress to database: track long-running job state in MongoDB, Firestore, or Redis
  - Job state machine: Pending → Running → Completed/Failed
  - Atomic updates: use database atomic operations to prevent race conditions
  - Track: current step/phase, percentage complete, items processed/total, start time, last updated, error count, retry count
  - Update frequency: balance freshness and database load, update every N items or every X seconds
  - MongoDB: use updateOne with $set for atomic updates
  - Firestore: use transactions or atomic field updates
  - Redis: use hashes for job state, HSET for updates, set TTL for auto-cleanup
  - Heartbeat: update updated_at periodically to show process is alive

- State Transitions
  - Atomic state changes: use compare-and-swap or transactions
  - Valid transitions: Pending → Running (claim job), Running → Completed (success), Running → Failed (permanent failure), Running → Pending (transient failure, retry)
  - Claim job pattern: atomically claim pending job with stale threshold check
  - On success: update to Completed, set completed_at, clear errors
  - On failure: update to Failed, set error_message, increment retry_count
  - On retry: reset to Pending if retry count below max, otherwise Failed
  - Idempotency key: store operation ID to prevent duplicate processing

- Process Lifecycle
  - Initialization: load configuration, connect to dependencies, verify connectivity, fail if critical resources unavailable
  - Running: accept and process work, monitor health, report metrics
  - Shutdown: stop accepting new work, drain existing work, close connections, exit cleanly

- Concurrency Control
  - Semaphore: limit concurrent operations (database connections, API calls)
  - Work queue: bounded channel for backpressure, block or reject when full
  - Rate limiting: limit operations per second using governor crate
  - Structured concurrency: use tokio::task::JoinSet to track spawned tasks

- Backpressure
  - Bounded channels: use bounded channels to apply backpressure
  - Reject when full: return error instead of unbounded queuing
  - Shed load: drop lowest priority work when overloaded
  - Monitoring: track queue depth and saturation

- Observability
  - Structured logging: use tracing crate for structured logs with spans
  - Metrics: export Prometheus metrics (request count, duration, errors, queue depth)
  - Tracing: distributed tracing with OpenTelemetry
  - Spans: instrument operations with tracing spans for context
  - Request ID: propagate request ID through operation for correlation

- Process Patterns
  - Worker: pull from queue, process, ack/nack based on result
  - Event processor: listen to events, process, publish results
  - Scheduled job: run at intervals with cron-like scheduling
  - Request-response: HTTP server handling requests synchronously
  - Stream processor: process continuous stream of data

- Resource Cleanup
  - RAII: use Rust's Drop trait for automatic cleanup
  - Connection pools: close all connections on shutdown
  - Flush buffers: flush logs, metrics before exit
  - Timeout cleanup: force cleanup after timeout

- Configuration
  - Environment variables: primary source for runtime config
  - Config files: for complex configuration, use TOML/YAML
  - Validation on load: validate all config at startup, fail fast if invalid
  - Secrets: load from secrets manager or environment, never hardcode
  - Reload support: support config reload without restart (when safe)

- Panic Handling
  - Catch panics: use std::panic::catch_unwind for critical sections
  - Panic hook: custom panic hook to log before process dies
  - Fail fast: for worker processes, crash and restart on panic
  - Graceful degradation: for servers, handle panics in request handlers without crashing process

- Timeouts
  - Set timeouts everywhere: database queries, HTTP requests, operations
  - Use tokio::time::timeout to wrap operations
  - Reasonable defaults: 5s for API calls, 30s for database queries, adjust based on SLA
  - Timeout as error: treat timeout as retriable error with backoff

- State Management
  - Shared state: use Arc<Mutex<T>> or Arc<RwLock<T>> for shared mutable state
  - Prefer message passing: use channels over shared state when possible
  - Lock-free when possible: use atomics for counters, flags
  - Avoid deadlocks: acquire locks in consistent order, use timeout on lock acquisition

- Monitoring and Alerting
  - Export metrics: request rate, error rate, duration percentiles, queue depth
  - Log errors: structured logs with error context
  - Health checks: automated health checks from orchestrator
  - Alerting: alert on error rate spike, high latency, failed health checks
  - SLOs: define and monitor Service Level Objectives

- Best Practices
  - Implement graceful shutdown
  - Use exponential backoff with jitter
  - Classify errors as retryable or not
  - Set timeouts on all operations
  - Persist progress to database with atomic updates
  - Use structured logging with tracing
  - Validate config at startup
  - Handle backpressure explicitly
  - Never use unbounded retries
  - Never ignore shutdown signals
  - Never block shutdown indefinitely
  - Never use unbounded queues
  - Never panic in production without recovery
  - Never hold locks across await points
