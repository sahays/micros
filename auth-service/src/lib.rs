pub mod config;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::sync::Arc;
use crate::config::Config;
use crate::middleware::{LoginRateLimiter, PasswordResetRateLimiter};
use crate::services::{EmailService, JwtService, MongoDb};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: MongoDb,
    pub email: EmailService,
    pub jwt: JwtService,
    pub redis: Arc<dyn crate::services::TokenBlacklist>,
    pub login_rate_limiter: LoginRateLimiter,
    pub password_reset_rate_limiter: PasswordResetRateLimiter,
}

pub async fn build_router(state: AppState) -> Result<Router, anyhow::Error> {
    // TODO: Add user routes
    // TODO: Add admin routes

    // Create login route with rate limiting
    let login_limiter = state.login_rate_limiter.clone();
    let login_route = Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .layer(from_fn(move |req, next| {
            middleware::rate_limit_middleware(login_limiter.clone(), req, next)
        }));

    // Create password reset request route with rate limiting
    let reset_request_limiter = state.password_reset_rate_limiter.clone();
    let reset_request_route = Router::new()
        .route(
            "/auth/password-reset/request",
            post(handlers::auth::request_password_reset),
        )
        .layer(from_fn(move |req, next| {
            middleware::rate_limit_middleware(reset_request_limiter.clone(), req, next)
        }));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/.well-known/jwks.json", get(handlers::well_known::jwks))
        // Authentication routes
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/verify", get(handlers::auth::verify_email))
        .route("/auth/introspect", post(handlers::auth::introspect))
        .route("/auth/google", get(handlers::auth::google_login))
        .route("/auth/google/callback", get(handlers::auth::google_callback))
        .merge(login_route)
        .merge(reset_request_route)
        .route(
            "/auth/password-reset/confirm",
            post(handlers::auth::confirm_password_reset),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .merge(
            Router::new()
                .route("/auth/logout", post(handlers::auth::logout))
                .route("/users/me", get(handlers::user::get_me).patch(handlers::user::update_me))
                .route("/users/me/password", post(handlers::user::change_password))
                .layer(from_fn_with_state(state.clone(), middleware::auth_middleware)),
        )
        .with_state(state)
        // Add CORS layer
        .layer(CorsLayer::permissive()) // TODO: Configure from config
        // Add tracing layer
        .layer(TraceLayer::new_for_http());

    Ok(app)
}

pub async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Check MongoDB connection
    state.db.health_check().await.map_err(|e| {
        tracing::error!("MongoDB health check failed: {}", e);
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!("MongoDB unhealthy: {}", e),
        )
    })?;

    // TODO: Check Redis connection

    Ok(axum::Json(serde_json::json!({
        "status": "healthy",
        "service": state.config.service_name,
        "version": state.config.service_version,
        "environment": format!("{:?}", state.config.environment),
        "checks": {
            "mongodb": "up"
        }
    })))
}

pub fn init_tracing(config: &Config) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
