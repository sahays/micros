use payment_service::{config::Config, Application};
use service_core::observability::logging::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().expect("Failed to load configuration");

    // Initialize tracing
    let otlp_endpoint = std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing(&config.service_name, "info", &otlp_endpoint);

    // Initialize metrics
    payment_service::services::metrics::init_metrics();

    let application = Application::build(config).await?;
    application.run_until_stopped().await?;

    Ok(())
}
