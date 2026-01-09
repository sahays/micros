use dotenvy::dotenv;
use secure_frontend::config::get_configuration;
use secure_frontend::services::auth_client::AuthClient;
use secure_frontend::startup::build_router;
use service_core::observability::logging::init_tracing;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let configuration = get_configuration().expect("Failed to read configuration.");

    // Initialize tracing using shared logic
    init_tracing("secure-frontend", "info", "http://tempo:4317");

    secure_frontend::services::metrics::init_metrics();

    let auth_client = Arc::new(AuthClient::new(configuration.auth_service.clone()));

    let app = build_router(auth_client);

    let address = format!(
        "{}:{}",
        configuration.server.host, configuration.server.port
    );
    let listener = tokio::net::TcpListener::bind(&address).await?;

    info!("Starting secure-frontend on {}", address);
    axum::serve(listener, app).await?;

    Ok(())
}
