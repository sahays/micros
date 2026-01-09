use dashmap::DashMap;
use governor::{
    clock::{Clock, DefaultClock},
    state::{keyed::DashMapStateStore, InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use service_core::axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use service_core::error::AppError;
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

use crate::services::AppTokenClaims;

/// Rate limiter for global/unkeyed use
pub type UnkeyedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Rate limiter keyed by IP address
pub type IpRateLimiter = Arc<RateLimiter<SocketAddr, DashMapStateStore<SocketAddr>, DefaultClock>>;

/// Rate limiter keyed by client ID with per-client quotas
pub type ClientRateLimiter = Arc<DashMap<String, UnkeyedRateLimiter>>;

/// Legacy aliases for backward compatibility
pub type LoginRateLimiter = UnkeyedRateLimiter;
pub type PasswordResetRateLimiter = UnkeyedRateLimiter;

/// Create a rate limiter for login attempts (unkeyed)
pub fn create_login_rate_limiter(attempts: u32, window_seconds: u64) -> LoginRateLimiter {
    // Ensure attempts is at least 1 to prevent division by zero and invalid NonZeroU32
    let attempts = attempts.max(1);

    let period = Duration::from_millis((window_seconds * 1000) / attempts as u64);
    let quota = Quota::with_period(period)
        .expect("Failed to create quota with valid period")
        .allow_burst(NonZeroU32::new(attempts).expect("attempts is guaranteed to be non-zero"));

    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter for password reset attempts (unkeyed)
pub fn create_password_reset_rate_limiter(
    attempts: u32,
    window_seconds: u64,
) -> PasswordResetRateLimiter {
    // Ensure attempts is at least 1 to prevent division by zero and invalid NonZeroU32
    let attempts = attempts.max(1);

    let period = Duration::from_millis((window_seconds * 1000) / attempts as u64);
    let quota = Quota::with_period(period)
        .expect("Failed to create quota with valid period")
        .allow_burst(NonZeroU32::new(attempts).expect("attempts is guaranteed to be non-zero"));

    Arc::new(RateLimiter::direct(quota))
}

/// Create a keyed rate limiter (by IP)
pub fn create_ip_rate_limiter(attempts: u32, window_seconds: u64) -> IpRateLimiter {
    // Ensure attempts is at least 1 to prevent division by zero and invalid NonZeroU32
    let attempts = attempts.max(1);

    let period = Duration::from_millis((window_seconds * 1000) / attempts as u64);
    let quota = Quota::with_period(period)
        .expect("Failed to create quota with valid period")
        .allow_burst(NonZeroU32::new(attempts).expect("attempts is guaranteed to be non-zero"));

    Arc::new(RateLimiter::dashmap(quota))
}

/// Create a new ClientRateLimiter
pub fn create_client_rate_limiter() -> ClientRateLimiter {
    Arc::new(DashMap::new())
}

/// Middleware for unkeyed rate limiting (Legacy)
pub async fn rate_limit_middleware(
    State(limiter): State<UnkeyedRateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    match limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(negative) => {
            let wait_time = negative.wait_time_from(DefaultClock::default().now());
            Err(AppError::TooManyRequests(
                "Too many requests. Please try again later.".to_string(),
                Some(wait_time.as_secs()),
            ))
        }
    }
}

/// Middleware for IP-based rate limiting
pub async fn ip_rate_limit_middleware(
    State(limiter): State<IpRateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Try to get IP from X-Forwarded-For
    let forwarded_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next()) // Get first IP
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok());

    let addr = if let Some(ip) = forwarded_ip {
        Some(SocketAddr::new(ip, 0))
    } else {
        // 2. Fallback to direct connection IP
        request
            .extensions()
            .get::<service_core::axum::extract::ConnectInfo<SocketAddr>>()
            .map(|service_core::axum::extract::ConnectInfo(addr)| *addr)
    };

    match addr {
        Some(addr) => match limiter.check_key(&addr) {
            Ok(_) => Ok(next.run(request).await),
            Err(negative) => {
                let wait_time = negative.wait_time_from(DefaultClock::default().now());
                Err(AppError::TooManyRequests(
                    "Too many requests from this IP. Please try again later.".to_string(),
                    Some(wait_time.as_secs()),
                ))
            }
        },
        None => {
            // If IP cannot be determined, proceed but log warning
            tracing::warn!("Could not determine IP for rate limiting");
            Ok(next.run(request).await)
        }
    }
}

/// Middleware for per-client rate limiting
pub async fn client_rate_limit_middleware(
    State(limiter_map): State<ClientRateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = request.extensions().get::<AppTokenClaims>();

    if let Some(claims) = claims {
        let client_id = &claims.client_id;
        let limit_per_min = claims.rate_limit_per_min;

        // Skip if limit is 0 (unlimited)
        if limit_per_min == 0 {
            return Ok(next.run(request).await);
        }

        // Get or create limiter for this client
        let limiter = limiter_map
            .entry(client_id.clone())
            .or_insert_with(|| create_login_rate_limiter(limit_per_min, 60))
            .clone();

        match limiter.check() {
            Ok(_) => Ok(next.run(request).await),
            Err(negative) => {
                let wait_time = negative.wait_time_from(DefaultClock::default().now());
                Err(AppError::TooManyRequests(
                    "Client rate limit exceeded".to_string(),
                    Some(wait_time.as_secs()),
                ))
            }
        }
    } else {
        // No app claims, proceed
        Ok(next.run(request).await)
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
