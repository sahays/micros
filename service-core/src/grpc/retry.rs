//! gRPC client retry utilities for service-to-service communication.
//!
//! Provides configurable retry logic with exponential backoff for gRPC calls.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tonic::{Code, Status};
use tracing::{info, warn};

/// Configuration for retry behavior.
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not including the initial attempt).
    pub max_retries: u32,
    /// Initial backoff duration before first retry.
    pub initial_backoff: Duration,
    /// Maximum backoff duration.
    pub max_backoff: Duration,
    /// Backoff multiplier for exponential backoff.
    pub backoff_multiplier: f64,
    /// Whether to add jitter to backoff duration.
    pub add_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with the specified max retries.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Create a config with no retries.
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Create a config for quick retries (smaller backoffs).
    pub fn quick() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(500),
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }

    /// Create a config for aggressive retries (more attempts, longer backoffs).
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }

    /// Calculate backoff duration for a given attempt.
    fn backoff_duration(&self, attempt: u32) -> Duration {
        let backoff =
            self.initial_backoff.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);
        let backoff_ms = backoff.min(self.max_backoff.as_millis() as f64) as u64;

        let mut duration = Duration::from_millis(backoff_ms);

        if self.add_jitter {
            // Add up to 25% jitter
            let jitter = (backoff_ms as f64 * 0.25 * rand_jitter()) as u64;
            duration += Duration::from_millis(jitter);
        }

        duration
    }
}

/// Simple pseudo-random jitter (0.0 to 1.0) without external dependencies.
fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// Determines if a gRPC status code is retryable.
pub fn is_retryable(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::Unavailable       // Service temporarily unavailable
        | Code::ResourceExhausted  // Rate limited
        | Code::Aborted           // Operation aborted, can retry
        | Code::DeadlineExceeded  // Timeout, can retry
        | Code::Unknown           // Unknown error, may be transient
        | Code::Internal // Internal error, may be transient
    )
}

/// Determines if a gRPC status code is definitely not retryable.
pub fn is_permanent_failure(status: &Status) -> bool {
    matches!(
        status.code(),
        Code::InvalidArgument    // Bad request
        | Code::NotFound          // Resource doesn't exist
        | Code::AlreadyExists     // Resource already exists
        | Code::PermissionDenied  // Not authorized
        | Code::Unauthenticated   // Not authenticated
        | Code::FailedPrecondition // State doesn't allow operation
        | Code::OutOfRange        // Invalid range
        | Code::Unimplemented // Method not implemented
    )
}

/// Execute a gRPC call with retry logic.
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation_name` - Name of the operation for logging
/// * `f` - The async function that performs the gRPC call
///
/// # Example
/// ```ignore
/// let result = retry_grpc_call(
///     &RetryConfig::default(),
///     "create_journal_entry",
///     || async {
///         client.create_journal_entry(request.clone()).await
///     }
/// ).await;
/// ```
pub async fn retry_grpc_call<F, Fut, T>(
    config: &RetryConfig,
    operation_name: &str,
    f: F,
) -> Result<T, Status>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Status>>,
{
    let mut attempt = 0;

    loop {
        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    info!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        "gRPC call succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(status) => {
                // Check if we should retry
                if attempt >= config.max_retries {
                    warn!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        code = ?status.code(),
                        message = status.message(),
                        "gRPC call failed after max retries"
                    );
                    return Err(status);
                }

                if is_permanent_failure(&status) {
                    warn!(
                        operation = operation_name,
                        code = ?status.code(),
                        message = status.message(),
                        "gRPC call failed with permanent error, not retrying"
                    );
                    return Err(status);
                }

                if !is_retryable(&status) {
                    warn!(
                        operation = operation_name,
                        code = ?status.code(),
                        message = status.message(),
                        "gRPC call failed with non-retryable error"
                    );
                    return Err(status);
                }

                // Calculate backoff and sleep
                let backoff = config.backoff_duration(attempt);
                warn!(
                    operation = operation_name,
                    attempt = attempt + 1,
                    code = ?status.code(),
                    message = status.message(),
                    backoff_ms = backoff.as_millis(),
                    "gRPC call failed, retrying after backoff"
                );

                sleep(backoff).await;
                attempt += 1;
            }
        }
    }
}

/// A wrapper that provides retry functionality for any gRPC client.
#[derive(Clone)]
pub struct RetryingClient<C> {
    inner: C,
    config: RetryConfig,
}

impl<C> RetryingClient<C> {
    /// Create a new retrying client wrapper.
    pub fn new(client: C, config: RetryConfig) -> Self {
        Self {
            inner: client,
            config,
        }
    }

    /// Create with default retry configuration.
    pub fn with_defaults(client: C) -> Self {
        Self::new(client, RetryConfig::default())
    }

    /// Get a reference to the inner client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Get a mutable reference to the inner client.
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// Get the retry configuration.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Execute an operation with retry.
    pub async fn execute<F, Fut, T>(&self, operation_name: &str, f: F) -> Result<T, Status>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, Status>>,
    {
        retry_grpc_call(&self.config, operation_name, f).await
    }
}

/// Extension trait for adding retry capability to any Result<T, Status>.
pub trait RetryExt<T> {
    /// Retry the operation if it fails with a retryable error.
    fn with_retry<F, Fut>(
        self,
        config: &RetryConfig,
        operation_name: &str,
        retry_fn: F,
    ) -> impl Future<Output = Result<T, Status>>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, Status>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff, Duration::from_millis(100));
    }

    #[test]
    fn test_backoff_duration() {
        let config = RetryConfig {
            add_jitter: false,
            ..Default::default()
        };

        assert_eq!(config.backoff_duration(0), Duration::from_millis(100));
        assert_eq!(config.backoff_duration(1), Duration::from_millis(200));
        assert_eq!(config.backoff_duration(2), Duration::from_millis(400));
    }

    #[test]
    fn test_is_retryable() {
        assert!(is_retryable(&Status::unavailable("service down")));
        assert!(is_retryable(&Status::resource_exhausted("rate limited")));
        assert!(is_retryable(&Status::deadline_exceeded("timeout")));
        assert!(!is_retryable(&Status::invalid_argument("bad request")));
        assert!(!is_retryable(&Status::not_found("not found")));
    }

    #[test]
    fn test_is_permanent_failure() {
        assert!(is_permanent_failure(&Status::invalid_argument("bad")));
        assert!(is_permanent_failure(&Status::not_found("missing")));
        assert!(is_permanent_failure(&Status::permission_denied("denied")));
        assert!(!is_permanent_failure(&Status::unavailable("down")));
    }

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let config = RetryConfig::default();
        let result = retry_grpc_call(&config, "test_op", || async { Ok::<_, Status>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_permanent_failure() {
        let config = RetryConfig::quick();
        let result = retry_grpc_call(&config, "test_op", || async {
            Err::<i32, _>(Status::not_found("not found"))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), Code::NotFound);
    }
}
