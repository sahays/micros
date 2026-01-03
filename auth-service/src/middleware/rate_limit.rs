use axum::{
    extract::{Request, State},
    http::{header, HeaderName, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::{Clock, DefaultClock},
    state::{keyed::DashMapStateStore, InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde_json::json;
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

/// Rate limiter for global/unkeyed use
pub type UnkeyedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Rate limiter keyed by IP address
pub type IpRateLimiter = Arc<RateLimiter<SocketAddr, DashMapStateStore<SocketAddr>, DefaultClock>>;

/// Legacy aliases for backward compatibility
pub type LoginRateLimiter = UnkeyedRateLimiter;
pub type PasswordResetRateLimiter = UnkeyedRateLimiter;

/// Create a rate limiter for login attempts (unkeyed)
pub fn create_login_rate_limiter(attempts: u32, window_seconds: u64) -> LoginRateLimiter {
    let quota = Quota::with_period(Duration::from_millis(
        (window_seconds * 1000) / attempts as u64,
    ))
    .unwrap()
    .allow_burst(NonZeroU32::new(attempts).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter for password reset attempts (unkeyed)
pub fn create_password_reset_rate_limiter(
    attempts: u32,
    window_seconds: u64,
) -> PasswordResetRateLimiter {
    let quota = Quota::with_period(Duration::from_millis(
        (window_seconds * 1000) / attempts as u64,
    ))
    .unwrap()
    .allow_burst(NonZeroU32::new(attempts).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Create a keyed rate limiter (by IP)
pub fn create_ip_rate_limiter(attempts: u32, window_seconds: u64) -> IpRateLimiter {
    let quota = Quota::with_period(Duration::from_millis(
        (window_seconds * 1000) / attempts as u64,
    ))
    .unwrap()
    .allow_burst(NonZeroU32::new(attempts).unwrap());

    Arc::new(RateLimiter::dashmap(quota))
}

/// Middleware for unkeyed rate limiting (Legacy)
pub async fn rate_limit_middleware(
    State(limiter): State<UnkeyedRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(_) => next.run(request).await,
        Err(negative) => {
            let wait_time = negative.wait_time_from(DefaultClock::default().now());
            let x_ratelimit_limit = HeaderName::from_static("x-ratelimit-limit");
            (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    (header::RETRY_AFTER, wait_time.as_secs().to_string()),
                    (x_ratelimit_limit.clone(), "unknown".to_string()),
                ],
                Json(json!({
                    "error": "Too many requests. Please try again later.",
                    "retry_after_seconds": wait_time.as_secs()
                })),
            )
                .into_response()
        }
    }
}

/// Middleware for IP-based rate limiting
pub async fn ip_rate_limit_middleware(
    State(limiter): State<IpRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    // Extract IP from extensions (populated by ConnectInfo)
    let addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| *addr);

    match addr {
        Some(addr) => match limiter.check_key(&addr) {
            Ok(_) => next.run(request).await,
            Err(negative) => {
                let wait_time = negative.wait_time_from(DefaultClock::default().now());
                let x_ratelimit_limit = HeaderName::from_static("x-ratelimit-limit");
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [
                        (header::RETRY_AFTER, wait_time.as_secs().to_string()),
                        (x_ratelimit_limit, "unknown".to_string()),
                    ],
                    Json(json!({
                        "error": "Too many requests from this IP. Please try again later.",
                        "retry_after_seconds": wait_time.as_secs()
                    })),
                )
                    .into_response()
            }
        },
        None => {
            // If IP cannot be determined, proceed but log warning
            tracing::warn!("Could not determine IP for rate limiting");
            next.run(request).await
        }
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
