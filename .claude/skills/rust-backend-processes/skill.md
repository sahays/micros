---
name: rust-backend-processes
description:
  Build robust Rust backend processes with retry logic, progress reporting, and graceful shutdown handling. Use when
  implementing long-running services, workers, or background processes requiring resilience and observability.
---

# Rust Backend Processes

## Graceful Shutdown

**Signal handling**: Use `tokio::signal` to handle SIGTERM, SIGINT.

**Shutdown pattern**:

- Receive shutdown signal
- Stop accepting new work
- Complete in-flight operations (with timeout)
- Clean up resources
- Exit gracefully

**Tokio shutdown**: Use `CancellationToken` or `tokio::sync::watch` for coordinated shutdown.

**Timeout on shutdown**: Don't wait forever. Force shutdown after grace period (30-60 seconds).

## Retry Strategies

**Exponential backoff**: Double delay between retries. Start at 100ms, cap at 30s.

**Max attempts**: Limit retry attempts (3-5 for transient errors, unlimited with backoff for critical operations).

**Jitter**: Add randomness to backoff to prevent thundering herd.

**Idempotency**: Ensure operations are safe to retry.

**Library**: Use `backoff` crate for retry logic with exponential backoff.

## Error Handling

**Transient vs permanent**: Retry transient errors (network, timeout), fail fast on permanent (auth, validation).

**Error classification**: Categorize errors as retryable or non-retryable.

**Error context**: Use `anyhow` for applications, `thiserror` for libraries. Add context to errors.

**Fail fast**: Don't retry non-retryable errors. Log and move on.

**Dead letter queue**: Move failed items to DLQ after max retries for manual inspection.

## Circuit Breaker

**Purpose**: Prevent cascading failures when dependency is down.

**States**: Closed (normal), Open (failing), Half-Open (testing recovery).

**Implementation**: Track failure rate. Open circuit after threshold. Test with single request in half-open state.

**Library**: Custom implementation or use existing patterns.

**Fast failure**: Return immediately when circuit is open instead of waiting for timeout.

## Progress Reporting

**Persist progress to database**: Track long-running job state in MongoDB, Firestore, or Redis.

**Job state machine**: Pending → Running → Completed/Failed

**Atomic updates**: Use database atomic operations to prevent race conditions.

**Progress tracking**:

- Current step/phase
- Percentage complete
- Items processed/total
- Start time, last updated
- Error count, retry count

**Status document pattern**:

```rust
struct JobStatus {
    id: String,
    state: JobState,  // Pending, Running, Completed, Failed
    progress: f32,    // 0.0 to 1.0
    current_step: String,
    error_message: Option<String>,
    started_at: DateTime,
    updated_at: DateTime,
    completed_at: Option<DateTime>,
}
```

**Update frequency**: Balance between freshness and database load. Update every N items or every X seconds.

**MongoDB**: Use `updateOne` with `$set` for atomic progress updates.

**Firestore**: Use transactions or atomic field updates.

**Redis**: Use hashes for job state, `HSET` for updates. Set TTL for auto-cleanup.

**Heartbeat**: Update `updated_at` periodically to show process is alive.

## State Transitions

**Atomic state changes**: Use compare-and-swap or transactions for state transitions.

**Valid transitions**:

- Pending → Running (claim job)
- Running → Completed (success)
- Running → Failed (permanent failure)
- Running → Pending (transient failure, retry)

**Claim job pattern** (MongoDB):

```rust
// Atomically claim pending job
db.find_one_and_update(
    { state: "Pending", claimed_at: { $lt: stale_threshold } },
    { $set: { state: "Running", claimed_at: now() } }
)
```

**On success**: Update to Completed, set `completed_at`, clear errors.

**On failure**: Update to Failed, set `error_message`, increment `retry_count`.

**On retry**: Reset to Pending if retry count below max, otherwise Failed.

**Idempotency key**: Store operation ID to prevent duplicate processing.

## Process Lifecycle

**Initialization phase**:

- Load configuration
- Connect to dependencies
- Verify connectivity
- Fail if critical resources unavailable

**Running phase**:

- Accept and process work
- Monitor health
- Report metrics

**Shutdown phase**:

- Stop accepting new work
- Drain existing work
- Close connections
- Exit cleanly

## Concurrency Control

**Semaphore**: Limit concurrent operations (database connections, API calls).

**Work queue**: Bounded channel for backpressure. Block or reject when full.

**Rate limiting**: Limit operations per second using `governor` crate.

**Structured concurrency**: Use `tokio::task::JoinSet` to track spawned tasks.

## Backpressure

**Bounded channels**: Use bounded channels to apply backpressure.

**Reject when full**: Return error instead of unbounded queuing.

**Shed load**: Drop lowest priority work when overloaded.

**Monitoring**: Track queue depth and saturation.

## Observability

**Structured logging**: Use `tracing` crate for structured logs with spans.

**Metrics**: Export Prometheus metrics - request count, duration, errors, queue depth.

**Tracing**: Distributed tracing with OpenTelemetry.

**Spans**: Instrument operations with tracing spans for context.

**Request ID**: Propagate request ID through operation for correlation.

## Process Patterns

**Worker pattern**: Pull from queue, process, ack/nack based on result.

**Event processor**: Listen to events, process, publish results.

**Scheduled job**: Run at intervals with cron-like scheduling.

**Request-response**: HTTP server handling requests synchronously.

**Stream processor**: Process continuous stream of data.

## Resource Cleanup

**RAII**: Use Rust's Drop trait for automatic cleanup.

**Defer cleanup**: Ensure cleanup happens even on early return or panic.

**Connection pools**: Close all connections on shutdown.

**Flush buffers**: Flush logs, metrics before exit.

**Timeout cleanup**: Don't wait forever for cleanup. Force after timeout.

## Configuration

**Environment variables**: Primary source for runtime config.

**Config files**: For complex configuration, use TOML/YAML.

**Validation on load**: Validate all config at startup. Fail fast if invalid.

**Secrets**: Load from secrets manager or environment, never hardcode.

**Reload support**: Support config reload without restart (when safe).

## Panic Handling

**Catch panics**: Use `std::panic::catch_unwind` for critical sections.

**Panic hook**: Custom panic hook to log before process dies.

**Fail fast**: For worker processes, crash and restart on panic (supervisor handles restart).

**Graceful degradation**: For servers, handle panics in request handlers without crashing process.

## Timeouts

**Set timeouts everywhere**: Database queries, HTTP requests, operations.

**Use tokio::time::timeout**: Wrap operations with timeout.

**Reasonable defaults**: 5s for API calls, 30s for database queries, adjust based on SLA.

**Timeout as error**: Treat timeout as retriable error with backoff.

## State Management

**Shared state**: Use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared mutable state.

**Prefer message passing**: Use channels over shared state when possible.

**Lock-free when possible**: Use atomics for counters, flags.

**Avoid deadlocks**: Acquire locks in consistent order, use timeout on lock acquisition.

## Monitoring and Alerting

**Export metrics**: Request rate, error rate, duration percentiles, queue depth.

**Log errors**: Structured logs with error context for debugging.

**Health checks**: Automated health checks from orchestrator.

**Alerting**: Alert on error rate spike, high latency, failed health checks.

**SLOs**: Define and monitor Service Level Objectives.

## Common Patterns

**Retry with backoff**:

```rust
use backoff::{ExponentialBackoff, retry};

retry(ExponentialBackoff::default(), || {
    risky_operation().map_err(|e| {
        if is_transient(e) {
            backoff::Error::Transient(e)
        } else {
            backoff::Error::Permanent(e)
        }
    })
})
```

**Graceful shutdown**:

```rust
use tokio::signal;
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let cloned_token = token.clone();

tokio::spawn(async move {
    signal::ctrl_c().await.unwrap();
    cloned_token.cancel();
});

// In workers
while !token.is_cancelled() {
    // Do work
}
```

**Progress reporting**:

```rust
async fn update_progress(
    db: &Database,
    job_id: &str,
    progress: f32,
    step: &str
) -> Result<()> {
    db.collection("jobs").update_one(
        doc! { "_id": job_id },
        doc! { "$set": {
            "progress": progress,
            "current_step": step,
            "updated_at": DateTime::now()
        }}
    ).await?;
    Ok(())
}
```

## Best Practices

**Do**:

- Implement graceful shutdown
- Use exponential backoff with jitter
- Classify errors as retryable or not
- Set timeouts on all operations
- Persist progress to database with atomic updates
- Use structured logging with tracing
- Validate config at startup
- Handle backpressure explicitly

**Avoid**:

- Unbounded retries
- No timeout on operations
- Ignoring shutdown signals
- Blocking shutdown indefinitely
- Unbounded queues
- Silent failures
- Panicking in production without recovery
- Holding locks across await points
