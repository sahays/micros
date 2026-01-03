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

use crate::config::Config;
use crate::middleware::{IpRateLimiter, LoginRateLimiter, PasswordResetRateLimiter};
use crate::services::{EmailService, JwtService, MongoDb};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: MongoDb,
    pub email: EmailService,
    pub jwt: JwtService,
    pub redis: Arc<dyn crate::services::TokenBlacklist>,
    pub login_rate_limiter: LoginRateLimiter,
    pub password_reset_rate_limiter: PasswordResetRateLimiter,
    pub app_token_rate_limiter: IpRateLimiter,
    pub client_rate_limiter: crate::middleware::ClientRateLimiter,
    pub ip_rate_limiter: IpRateLimiter,
}

pub async fn build_router(state: AppState) -> Result<Router, anyhow::Error> {
    // TODO: Add user routes

    // Admin routes
    let admin_routes = Router::new()
        .route("/auth/admin/clients", post(handlers::admin::create_client))
        .route(
            "/auth/admin/clients/:client_id/rotate",
            post(handlers::admin::rotate_client_secret),
        )
        .route(
            "/auth/admin/clients/:client_id",
            axum::routing::delete(handlers::admin::revoke_client),
        )
        .layer(from_fn_with_state(
            state.clone(),
            middleware::admin_auth_middleware,
        ));

    // Create login route with rate limiting
    let login_limiter = state.login_rate_limiter.clone();
    let login_route = Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .layer(from_fn_with_state(
            login_limiter,
            middleware::rate_limit_middleware,
        ));

    // Create password reset request route with rate limiting
    let reset_request_limiter = state.password_reset_rate_limiter.clone();
    let reset_request_route = Router::new()
        .route(
            "/auth/password-reset/request",
            post(handlers::auth::request_password_reset),
        )
        .layer(from_fn_with_state(
            reset_request_limiter,
            middleware::rate_limit_middleware,
        ));

    // Create app token route with rate limiting
    let app_token_limiter = state.app_token_rate_limiter.clone();
    let app_token_route = Router::new()
        .route("/auth/app/token", post(handlers::app::app_token))
        .layer(from_fn_with_state(
            app_token_limiter,
            middleware::ip_rate_limit_middleware,
        ));

    // Create global IP rate limiter
    let ip_limiter = state.ip_rate_limiter.clone();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/.well-known/jwks.json", get(handlers::well_known::jwks))
        // Authentication routes
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/verify", get(handlers::auth::verify_email))
        .route("/auth/introspect", post(handlers::auth::introspect))
        .route("/auth/google", get(handlers::auth::google_login))
        .route(
            "/auth/google/callback",
            get(handlers::auth::google_callback),
        )
        .merge(app_token_route)
        .merge(login_route)
        .merge(reset_request_route)
        .merge(admin_routes)
        .route(
            "/auth/password-reset/confirm",
            post(handlers::auth::confirm_password_reset),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .merge(
            Router::new()
                .route("/auth/logout", post(handlers::auth::logout))
                .route(
                    "/users/me",
                    get(handlers::user::get_me).patch(handlers::user::update_me),
                )
                .route("/users/me/password", post(handlers::user::change_password))
                .layer(from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                )),
        )
        .with_state(state)
        // Global IP rate limiting
        .layer(from_fn_with_state(
            ip_limiter,
            middleware::ip_rate_limit_middleware,
        ))
        // Add tracing middleware for request_id
        .layer(from_fn(middleware::request_id_middleware))
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
        tracing::error!(error = %e, "MongoDB health check failed");
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
