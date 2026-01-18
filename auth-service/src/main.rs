//! Auth Service v2 - Main entry point.

use auth_service::{build_router, config::AuthConfig, db, services, AppState};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration first (before tracing init)
    let config = AuthConfig::from_env()?;

    // Initialize tracing with JSON format for PLG stack
    init_tracing(&config);

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = ?config.environment,
        "Starting auth-service v2"
    );

    // Create PostgreSQL connection pool
    let pool = db::create_pool(&config.database).await?;
    tracing::info!("PostgreSQL connection pool created");

    // Run migrations
    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    // Create database wrapper
    let database = services::Database::new(pool);

    // Create JWT service
    let jwt = services::JwtService::new(&config.jwt)?;
    tracing::info!("JWT service initialized");

    // Create Redis client
    let redis = Arc::new(services::RedisService::new(&config.redis).await?)
        as Arc<dyn services::TokenBlacklist>;
    tracing::info!("Redis connection established");

    // Create email service
    let email =
        Arc::new(services::EmailService::new(&config.gmail)?) as Arc<dyn services::EmailProvider>;
    tracing::info!("Email service initialized");

    // Build application state
    let state = AppState {
        config: config.clone(),
        db: database,
        email,
        jwt,
        redis,
    };

    // Build router
    let app = build_router(state).await?;

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Service shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {}", e);
            }
        }
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
}

/// Initialize tracing with JSON format for PLG stack.
///
/// When OTLP_ENDPOINT is configured, traces are exported to Tempo.
/// Logs are always output as JSON to stdout for Promtail collection.
fn init_tracing(config: &AuthConfig) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

    // Try to set up OpenTelemetry if OTLP endpoint is configured
    if let Some(ref otlp_endpoint) = config.otlp_endpoint {
        let otlp_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(otlp_endpoint);

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(otlp_exporter)
            .with_trace_config(
                sdktrace::Config::default().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.service_name.clone()),
                    KeyValue::new("service.version", config.service_version.clone()),
                ])),
            )
            .install_batch(runtime::Tokio)
        {
            Ok(tracer) => {
                let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(telemetry)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_file(true)
                            .with_line_number(true)
                            .with_target(true)
                            .json()
                            .flatten_event(true),
                    )
                    .init();
                return;
            }
            Err(e) => {
                eprintln!(
                    "Failed to initialize OTLP tracer (endpoint: {}): {}. Falling back to JSON-only logging.",
                    otlp_endpoint, e
                );
            }
        }
    }

    // Fallback: JSON logging without OpenTelemetry
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .json()
                .flatten_event(true),
        )
        .init();
}
