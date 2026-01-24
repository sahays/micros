//! Reconciliation Service entry point.

use reconciliation_service::config::ReconciliationConfig;
use reconciliation_service::services::init_metrics;
use reconciliation_service::startup::Application;

use service_core::observability::init_tracing;
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = ReconciliationConfig::from_env().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::io::Error::other(format!("Configuration error: {}", e))
    })?;

    // Initialize tracing
    let otlp_endpoint = config
        .otlp_endpoint
        .clone()
        .unwrap_or_else(|| "http://tempo:4317".to_string());
    init_tracing(&config.service_name, &config.log_level, &otlp_endpoint);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        otlp_endpoint = %otlp_endpoint,
        "Starting reconciliation-service"
    );

    // Initialize metrics
    init_metrics();

    // Log configuration (mask sensitive values)
    tracing::info!(
        service_name = %config.service_name,
        http_port = %config.common.port,
        grpc_port = %(config.common.port + 1),
        db_max_connections = %config.database.max_connections,
        db_min_connections = %config.database.min_connections,
        ledger_service_url = %config.ledger_service.url,
        genai_service_url = %config.genai_service.url,
        document_service_url = %config.document_service.url,
        "Configuration loaded"
    );

    // Build and run application
    let app = Application::build(config).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to build application");
        std::io::Error::other(format!("Application build error: {}", e))
    })?;

    // Run with graceful shutdown
    tokio::select! {
        result = app.run_until_stopped() => {
            if let Err(e) = result {
                tracing::error!(error = %e, "Application error");
                return Err(e);
            }
        }
        _ = shutdown_signal() => {
            tracing::info!("Graceful shutdown initiated");
        }
    }

    tracing::info!("Service shutdown complete");
    Ok(())
}
