use payment_service::config::Config;
use payment_service::startup::Application;
use service_core::observability::init_tracing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().expect("Failed to load configuration");

    // Initialize tracing
    init_tracing(&config.service_name, "info");

    // Build and run the application
    let application = Application::build(config).await?;

    tracing::info!(
        "Payment service starting - HTTP on port {}, gRPC on port {}",
        application.http_port(),
        application.grpc_port()
    );

    application.run_until_stopped().await?;

    Ok(())
}
