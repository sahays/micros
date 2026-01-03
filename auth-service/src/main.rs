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
use crate::services::MongoDb;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: MongoDb,
}

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

    // TODO: Initialize Redis
    // TODO: Load JWT keys

    // Create application state
    let state = AppState {
        config: config.clone(),
        db,
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

async fn build_router(state: AppState) -> Result<Router, anyhow::Error> {
    // TODO: Add authentication routes
    // TODO: Add user routes
    // TODO: Add admin routes
    // TODO: Add middleware (CORS, auth, rate limiting)

    let app = Router::new()
        .route("/health", get(health_check))
        .with_state(state)
        // Add CORS layer
        .layer(CorsLayer::permissive()) // TODO: Configure from config
        // Add tracing layer
        .layer(TraceLayer::new_for_http());

    Ok(app)
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Check MongoDB connection
    state
        .db
        .health_check()
        .await
        .map_err(|e| {
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
