use payment_service::config::Config;
use payment_service::grpc::{
    proto::{payment_service_server::PaymentServiceServer, FILE_DESCRIPTOR_SET},
    PaymentGrpcService,
};
use payment_service::Application;
use service_core::observability::logging::init_tracing;
use std::net::SocketAddr;
use tokio::signal;
use tonic::transport::Server as GrpcServer;

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

    tracing::info!("Shutdown signal received");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().expect("Failed to load configuration");

    // Initialize tracing
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing(&config.service_name, "info", &otlp_endpoint);

    // Initialize metrics
    payment_service::services::metrics::init_metrics();

    // Build the HTTP application (REST API for webhooks and health)
    let application = Application::build(config.clone()).await?;
    let http_port = application.port();

    // Get AppState for gRPC service
    let app_state = application.state();

    // gRPC server configuration
    let grpc_port = config.server.grpc_port;
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));

    // Create gRPC service
    let payment_service = PaymentGrpcService::new(app_state);

    // gRPC health service
    let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<PaymentServiceServer<PaymentGrpcService>>()
        .await;

    // Reflection service for debugging
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|e| anyhow::anyhow!("Failed to build reflection service: {}", e))?;

    tracing::info!(
        "Payment service starting - HTTP on port {}, gRPC on port {}",
        http_port,
        grpc_port
    );

    // Build gRPC server
    let grpc_server = GrpcServer::builder()
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(PaymentServiceServer::new(payment_service))
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    // Run both servers concurrently
    tokio::select! {
        result = application.run_until_stopped() => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
        result = grpc_server => {
            if let Err(e) = result {
                tracing::error!("gRPC server error: {}", e);
            }
        }
    }

    Ok(())
}
