use dashmap::DashMap;
use governor::{
    clock::{Clock, DefaultClock},
    state::{keyed::DashMapStateStore, InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use crate::error::AppError;
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

/// Rate limiter for global/unkeyed use
pub type UnkeyedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Rate limiter keyed by IP address
pub type IpRateLimiter = Arc<RateLimiter<SocketAddr, DashMapStateStore<SocketAddr>, DefaultClock>>;

/// Rate limiter keyed by client ID with per-client quotas
pub type ClientRateLimiter = Arc<DashMap<String, UnkeyedRateLimiter>>;

/// Trait for extracting rate limit info from request extensions
pub trait HasRateLimitInfo: Send + Sync + 'static {
    fn client_id(&self) -> String;
    fn rate_limit_per_min(&self) -> u32;
}

/// Create a rate limiter for login attempts (unkeyed)
pub fn create_unkeyed_rate_limiter(attempts: u32, window_seconds: u64) -> UnkeyedRateLimiter {
    let attempts = attempts.max(1);
    let period = Duration::from_millis((window_seconds * 1000) / attempts as u64);
    let quota = Quota::with_period(period)
        .expect("Failed to create quota with valid period")
        .allow_burst(NonZeroU32::new(attempts).expect("attempts is guaranteed to be non-zero"));

    Arc::new(RateLimiter::direct(quota))
}

/// Create a keyed rate limiter (by IP)
pub fn create_ip_rate_limiter(attempts: u32, window_seconds: u64) -> IpRateLimiter {
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

/// Middleware for unkeyed rate limiting
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
    let forwarded_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok());

    let addr = if let Some(ip) = forwarded_ip {
        Some(SocketAddr::new(ip, 0))
    } else {
        request
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|axum::extract::ConnectInfo(addr)| *addr)
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
            tracing::warn!("Could not determine IP for rate limiting");
            Ok(next.run(request).await)
        }
    }
}

/// Middleware for per-client rate limiting
pub async fn client_rate_limit_middleware<T>(
    State(limiter_map): State<ClientRateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> 
where T: HasRateLimitInfo + Clone {
    let info = request.extensions().get::<T>();

    if let Some(info) = info {
        let client_id = info.client_id();
        let limit_per_min = info.rate_limit_per_min();

        if limit_per_min == 0 {
            return Ok(next.run(request).await);
        }

        let limiter = limiter_map
            .entry(client_id.clone())
            .or_insert_with(|| create_unkeyed_rate_limiter(limit_per_min, 60))
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
        Ok(next.run(request).await)
    }
}