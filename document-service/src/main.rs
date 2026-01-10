use document_service::config::DocumentConfig;
use document_service::startup::Application;
use service_core::observability::init_tracing;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing (logs, etc.) from service-core
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing("document-service", "info", &otlp_endpoint);

    let config = DocumentConfig::load().map_err(|e| {
        tracing::error!("Failed to load configuration: {}", e);
        std::io::Error::other(format!("Configuration error: {}", e))
    })?;

    let app = Application::build(config).await.map_err(|e| {
        tracing::error!("Failed to build application: {}", e);
        std::io::Error::other(format!("Application build error: {}", e))
    })?;

    app.run_until_stopped().await
}
