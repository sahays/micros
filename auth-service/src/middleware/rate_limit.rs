use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde_json::json;
use std::{num::NonZeroU32, sync::Arc, time::Duration};

/// Rate limiter for login endpoint
pub type LoginRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Rate limiter for password reset endpoint
pub type PasswordResetRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a rate limiter for login attempts
pub fn create_login_rate_limiter(attempts: u32, window_seconds: u64) -> LoginRateLimiter {
    let quota = Quota::with_period(Duration::from_secs(window_seconds / attempts as u64))
        .unwrap()
        .allow_burst(NonZeroU32::new(attempts).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter for password reset attempts
pub fn create_password_reset_rate_limiter(
    attempts: u32,
    window_seconds: u64,
) -> PasswordResetRateLimiter {
    let quota = Quota::with_period(Duration::from_secs(window_seconds / attempts as u64))
        .unwrap()
        .allow_burst(NonZeroU32::new(attempts).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Middleware to rate limit requests
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(request).await,
        Err(_) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Too many requests. Please try again later."
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = create_login_rate_limiter(5, 900);
        assert!(limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = create_login_rate_limiter(3, 60);

        // First 3 requests should succeed
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());

        // 4th request should be rate limited
        assert!(limiter.check().is_err());
    }
}
