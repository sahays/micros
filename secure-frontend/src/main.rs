use dotenvy::dotenv;
use secure_frontend::config::get_configuration;
use secure_frontend::services::auth_client::AuthClient;
use secure_frontend::services::document_client::DocumentClient;
use secure_frontend::startup::build_router;
use secure_frontend::AppState;
use service_core::observability::logging::init_tracing;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let configuration = get_configuration().map_err(|e| {
        eprintln!("Failed to read configuration: {}", e);
        anyhow::anyhow!("Configuration error: {}", e)
    })?;

    // Initialize tracing using shared logic
    init_tracing("secure-frontend", "info", "http://tempo:4317");

    secure_frontend::services::metrics::init_metrics();

    // Initialize service clients
    let auth_client = Arc::new(AuthClient::new(configuration.auth_service.clone()));
    let document_client = Arc::new(DocumentClient::new(configuration.document_service.clone()));

    // Create shared application state
    let app_state = AppState::new(auth_client, document_client);

    let app = build_router(app_state);

    let address = format!(
        "{}:{}",
        configuration.server.host, configuration.server.port
    );
    let listener = tokio::net::TcpListener::bind(&address).await.map_err(|e| {
        tracing::error!("Failed to bind TCP listener to {}: {}", address, e);
        anyhow::anyhow!("Failed to bind to address {}: {}", address, e)
    })?;

    info!("Starting secure-frontend on {}", address);
    axum::serve(listener, app).await.map_err(|e| {
        tracing::error!("Server error: {}", e);
        anyhow::anyhow!("Server error: {}", e)
    })?;

    Ok(())
}
