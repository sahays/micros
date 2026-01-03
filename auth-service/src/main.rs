mod config;
mod handlers;
mod middleware;
mod models;
mod services;
mod utils;

use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;

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
    // TODO: Initialize MongoDB
    // TODO: Initialize Redis

    // TODO: Load JWT keys

    // Build application router
    let app = build_router(config.clone()).await?;

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

async fn build_router(_config: Config) -> Result<Router, anyhow::Error> {
    // TODO: Add authentication routes
    // TODO: Add user routes
    // TODO: Add admin routes
    // TODO: Add middleware (CORS, auth, rate limiting)

    let app = Router::new()
        .route("/health", get(health_check))
        // Add CORS layer
        .layer(CorsLayer::permissive()) // TODO: Configure from config
        // Add tracing layer
        .layer(TraceLayer::new_for_http());

    Ok(app)
}

async fn health_check() -> &'static str {
    // TODO: Check MongoDB connection
    // TODO: Check Redis connection
    "OK"
}

fn init_tracing(config: &Config) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(&config.log_level)
        });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();
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
