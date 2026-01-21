use genai_service::config::GenaiConfig;
use genai_service::grpc::{
    proto::{gen_ai_service_server::GenAiServiceServer, FILE_DESCRIPTOR_SET},
    GenaiGrpcService,
};
use genai_service::services::providers::gemini::{GeminiConfig, GeminiTextProvider};
use genai_service::services::providers::TextProvider;
use genai_service::services::{get_metrics, init_metrics, DocumentFetcher, GenaiDb};
use genai_service::startup::AppState;

use axum::{
    extract::State, http::StatusCode, middleware, response::IntoResponse, routing::get, Json,
    Router,
};
use serde_json::json;
use service_core::middleware::metrics::metrics_middleware;
use service_core::middleware::tracing::request_id_middleware;
use service_core::observability::init_tracing;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tonic::transport::Server as GrpcServer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct HealthState {
    db: GenaiDb,
}

async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => {
            tracing::debug!("Health check passed");
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "service": "genai-service",
                    "version": env!("CARGO_PKG_VERSION")
                })),
            )
        }
        Err(e) => {
            tracing::warn!(error = %e, "Health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "unhealthy",
                    "service": "genai-service",
                    "error": e.to_string()
                })),
            )
        }
    }
}

async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => {
            tracing::debug!("Readiness check passed");
            StatusCode::OK
        }
        Err(e) => {
            tracing::warn!(error = %e, "Readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn metrics_handler() -> impl IntoResponse {
    let metrics = get_metrics();
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        metrics,
    )
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

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing("genai-service", "info", &otlp_endpoint);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        otlp_endpoint = %otlp_endpoint,
        "Starting genai-service"
    );

    // Initialize metrics
    init_metrics();

    let config = GenaiConfig::load().map_err(|e| {
        tracing::error!(error = %e, "Failed to load configuration");
        std::io::Error::other(format!("Configuration error: {}", e))
    })?;

    // Log configuration (with masked sensitive values)
    tracing::info!(
        mongodb_database = %config.mongodb.database,
        text_model = %config.models.text_model,
        audio_model = %config.models.audio_model,
        video_model = %config.models.video_model,
        document_service_url = %config.document_service.grpc_url,
        http_port = %config.common.port,
        grpc_port = %(config.common.port + 1),
        "Configuration loaded"
    );

    // Connect to database
    tracing::info!("Connecting to MongoDB...");
    let db = GenaiDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to connect to MongoDB");
            std::io::Error::other(format!("Database connection error: {}", e))
        })?;

    db.initialize_indexes().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to initialize database indexes");
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
        provider = "gemini",
        "Initialized text provider"
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

    // HTTP health endpoint for Docker/K8s probes with middleware
    let health_router = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/metrics", get(metrics_handler))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(metrics_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(health_state);

    let health_port = config.common.port;
    let health_addr = SocketAddr::from(([0, 0, 0, 0], health_port));
    let health_listener = TcpListener::bind(health_addr).await.map_err(|e| {
        tracing::error!(error = %e, addr = %health_addr, "Failed to bind health listener");
        e
    })?;
    tracing::info!(port = %health_port, "HTTP server listening");

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

    tracing::info!(port = %grpc_port, "gRPC server listening");

    // Build gRPC server
    let grpc_server = GrpcServer::builder()
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(GenAiServiceServer::new(genai_service))
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    tracing::info!(
        service = "genai-service",
        version = env!("CARGO_PKG_VERSION"),
        http_port = %health_port,
        grpc_port = %grpc_port,
        "Service ready to accept connections"
    );

    // Run both servers concurrently
    tokio::select! {
        result = axum::serve(health_listener, health_router) => {
            if let Err(e) = result {
                tracing::error!(error = %e, "HTTP server error");
            }
        }
        result = grpc_server => {
            if let Err(e) = result {
                tracing::error!(error = %e, "gRPC server error");
            }
        }
    }

    tracing::info!("Service shutdown complete");
    Ok(())
}
