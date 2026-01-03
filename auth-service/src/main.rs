use auth_service::{
    build_router,
    config::Config,
    init_tracing,
    middleware,
    services::{EmailService, JwtService, MongoDb},
    AppState,
};
use std::net::SocketAddr;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load configuration - fail fast if invalid
    let config = Config::from_env()?;

    // Initialize tracing/logging
    init_tracing(&config);

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

    // Initialize email service
    let email = EmailService::new(&config.gmail)?;
    tracing::info!("Email service initialized");

    // Initialize JWT service
    let jwt = JwtService::new(&config.jwt)?;
    tracing::info!("JWT service initialized");

    // Initialize rate limiters
    let login_rate_limiter = middleware::create_login_rate_limiter(
        config.rate_limit.login_attempts,
        config.rate_limit.login_window_seconds,
    );
    let password_reset_rate_limiter = middleware::create_password_reset_rate_limiter(
        config.rate_limit.password_reset_attempts,
        config.rate_limit.password_reset_window_seconds,
    );
    tracing::info!(
        "Rate limiters initialized: Login ({} attempts/{}s), Password Reset ({} attempts/{}s)",
        config.rate_limit.login_attempts,
        config.rate_limit.login_window_seconds,
        config.rate_limit.password_reset_attempts,
        config.rate_limit.password_reset_window_seconds
    );

    // TODO: Initialize Redis

    // Create application state
    let state = AppState {
        config: config.clone(),
        db,
        email,
        jwt,
        login_rate_limiter,
        password_reset_rate_limiter,
    };

    // Build application router
    let app = build_router(state).await?;

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
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