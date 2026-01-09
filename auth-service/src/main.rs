use auth_service::{
    build_router,
    config::AuthConfig,
    services::{EmailService, JwtService, MongoDb, RedisService},
    AppState,
};
use service_core::middleware::rate_limit::{
    create_client_rate_limiter, create_ip_rate_limiter,
};
use service_core::observability::logging::init_tracing;
use std::net::SocketAddr;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), service_core::error::AppError> {
    // Load configuration - fail fast if invalid
    let config = AuthConfig::from_env()?;

    // Initialize tracing/logging using shared logic
    init_tracing(
        &config.service_name,
        &config.log_level,
        "http://tempo:4317" // In production this would come from config
    );

    // Initialize metrics
    auth_service::services::metrics::init_metrics();

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = ?config.environment,
        "Starting authentication service"
    );

    // Initialize database connections
    tracing::info!("Initializing database connections");
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database).await?;

    // Create indexes
    db.initialize_indexes().await?;
    tracing::info!("Database initialized successfully");

    // Initialize Redis service
    let redis = RedisService::new(&config.redis).await?;
    tracing::info!("Redis service initialized");

    // Initialize email service
    let email = EmailService::new(&config.gmail)?;
    let email = std::sync::Arc::new(email);
    tracing::info!("Email service initialized");

    // Initialize JWT service
    let jwt = JwtService::new(&config.jwt)?;
    tracing::info!("JWT service initialized");

    // Initialize rate limiters using shared logic
    let login_rate_limiter = create_ip_rate_limiter(
        config.rate_limit.login_attempts,
        config.rate_limit.login_window_seconds,
    );
    let register_rate_limiter = create_ip_rate_limiter(
        config.rate_limit.register_attempts,
        config.rate_limit.register_window_seconds,
    );
    let password_reset_rate_limiter = create_ip_rate_limiter(
        config.rate_limit.password_reset_attempts,
        config.rate_limit.password_reset_window_seconds,
    );
    let ip_rate_limiter = create_ip_rate_limiter(
        config.rate_limit.global_ip_limit,
        config.rate_limit.global_ip_window_seconds,
    );
    let app_token_rate_limiter = create_ip_rate_limiter(
        config.rate_limit.app_token_limit,
        config.rate_limit.app_token_window_seconds,
    );
    let client_rate_limiter = create_client_rate_limiter();
    tracing::info!(
        "Rate limiters initialized: Login, Register, Password Reset, App Token, Client, and Global IP"
    );

    // Initialize services
    let redis = std::sync::Arc::new(redis);
    let auth_service = auth_service::services::AuthService::new(
        db.clone(),
        email.clone(),
        jwt.clone(),
        redis.clone(),
    );
    let admin_service = auth_service::services::admin::AdminService::new(db.clone(), redis.clone());

    // Create application state
    let state = AppState {
        config: config.clone(),
        db,
        email,
        jwt,
        auth_service,
        admin_service,
        redis,
        login_rate_limiter,
        register_rate_limiter,
        password_reset_rate_limiter,
        app_token_rate_limiter,
        client_rate_limiter,
        ip_rate_limiter,
    };
    // Build application router
    let app = build_router(state).await?;

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));

    let service_span = tracing::info_span!(
        "service",
        service = %config.service_name,
        version = %config.service_version,
        environment = ?config.environment,
    );
    let _guard = service_span.enter();

    tracing::info!(address = %addr, "Listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    service_core::axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("Service shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }

    // Give in-flight requests 30 seconds to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
}