use genai_service::config::GenaiConfig;
use genai_service::grpc::{
    proto::{gen_ai_service_server::GenAiServiceServer, FILE_DESCRIPTOR_SET},
    GenaiGrpcService,
};
use genai_service::services::providers::gemini::{GeminiConfig, GeminiTextProvider};
use genai_service::services::providers::TextProvider;
use genai_service::services::{DocumentFetcher, GenaiDb};
use genai_service::startup::AppState;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use service_core::observability::init_tracing;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tonic::transport::Server as GrpcServer;

#[derive(Clone)]
struct HealthState {
    db: GenaiDb,
}

async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "service": "genai-service",
                "version": env!("CARGO_PKG_VERSION")
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "service": "genai-service",
                "error": e.to_string()
            })),
        ),
    }
}

async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

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
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing("genai-service", "info", &otlp_endpoint);

    let config = GenaiConfig::load().map_err(|e| {
        tracing::error!("Failed to load configuration: {}", e);
        std::io::Error::other(format!("Configuration error: {}", e))
    })?;

    // Connect to database
    let db = GenaiDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to MongoDB: {}", e);
            std::io::Error::other(format!("Database connection error: {}", e))
        })?;

    db.initialize_indexes().await.map_err(|e| {
        tracing::error!("Failed to initialize database indexes: {}", e);
        std::io::Error::other(format!("Database initialization error: {}", e))
    })?;

    // Initialize Gemini text provider
    let gemini_config = GeminiConfig {
        api_key: config.google.api_key.clone(),
        model: config.models.text_model.clone(),
    };
    let text_provider: Arc<dyn TextProvider> = Arc::new(GeminiTextProvider::new(gemini_config));

    tracing::info!(
        model = %config.models.text_model,
        "Initialized Gemini text provider"
    );

    // Initialize document fetcher
    let document_fetcher = DocumentFetcher::new(&config.document_service.grpc_url);
    tracing::info!(
        endpoint = %config.document_service.grpc_url,
        "Initialized document fetcher"
    );

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        text_provider,
        document_fetcher,
    };

    let health_state = HealthState { db };

    // HTTP health endpoint for Docker/K8s probes
    let health_router = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .with_state(health_state);

    let health_port = config.common.port;
    let health_addr = SocketAddr::from(([0, 0, 0, 0], health_port));
    let health_listener = TcpListener::bind(health_addr).await.map_err(|e| {
        tracing::error!("Failed to bind health listener to {}: {}", health_addr, e);
        e
    })?;
    tracing::info!("Health endpoint listening on port {}", health_port);

    // gRPC server on port + 1
    let grpc_port = health_port + 1;
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));

    // Create gRPC services
    let genai_service = GenaiGrpcService::new(state);

    // gRPC health service
    let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<GenAiServiceServer<GenaiGrpcService>>()
        .await;

    // Reflection service for debugging
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|e| std::io::Error::other(format!("Failed to build reflection service: {}", e)))?;

    tracing::info!("gRPC server listening on port {}", grpc_port);

    // Build gRPC server
    let grpc_server = GrpcServer::builder()
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(GenAiServiceServer::new(genai_service))
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    // Run both servers concurrently
    tokio::select! {
        result = axum::serve(health_listener, health_router) => {
            if let Err(e) = result {
                tracing::error!("Health server error: {}", e);
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
