//! Application startup and lifecycle management.
//!
//! This module provides the minimal HTTP server (health/metrics) and gRPC server
//! for the genai service. All business logic is exposed via gRPC.

use crate::config::GenaiConfig;
use crate::grpc::{
    proto::{gen_ai_service_server::GenAiServiceServer, FILE_DESCRIPTOR_SET},
    GenaiGrpcService,
};
use crate::services::providers::gemini::{GeminiConfig, GeminiTextProvider};
use crate::services::providers::TextProvider;
use crate::services::{DocumentFetcher, GenaiDb};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use service_core::error::AppError;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: GenaiConfig,
    pub db: GenaiDb,
    pub text_provider: Arc<dyn TextProvider>,
    pub document_fetcher: DocumentFetcher,
}

/// State for health check endpoints.
#[derive(Clone)]
struct HealthState {
    db: GenaiDb,
}

/// Health check endpoint for Docker/K8s liveness probes.
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

/// Readiness check endpoint for K8s readiness probes.
async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Application container for managing server lifecycle.
pub struct Application {
    http_port: u16,
    grpc_port: u16,
    http_listener: TcpListener,
    grpc_listener: TcpListener,
    state: AppState,
}

impl Application {
    /// Build the application with the given configuration.
    pub async fn build(config: GenaiConfig) -> Result<Self, AppError> {
        // Connect to database
        let db = GenaiDb::connect(&config.mongodb.uri, &config.mongodb.database)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect to MongoDB: {}", e);
                e
            })?;

        db.initialize_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            e
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
            db,
            text_provider,
            document_fetcher,
        };

        // Bind HTTP listener (port 0 = random port for testing)
        let http_addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
        let http_listener = TcpListener::bind(http_addr).await.map_err(|e| {
            tracing::error!("Failed to bind HTTP listener to {}: {}", http_addr, e);
            AppError::from(e)
        })?;
        let http_port = http_listener.local_addr()?.port();

        // Bind gRPC listener (port 0 = random port for testing)
        let grpc_listener = TcpListener::bind("0.0.0.0:0").await.map_err(|e| {
            tracing::error!("Failed to bind gRPC listener: {}", e);
            AppError::from(e)
        })?;
        let grpc_port = grpc_listener.local_addr()?.port();

        tracing::info!(
            "GenAI service: HTTP on port {}, gRPC on port {}",
            http_port,
            grpc_port
        );

        Ok(Self {
            http_port,
            grpc_port,
            http_listener,
            grpc_listener,
            state,
        })
    }

    /// Get the HTTP port the server is listening on.
    pub fn http_port(&self) -> u16 {
        self.http_port
    }

    /// Get the gRPC port the server is listening on.
    pub fn grpc_port(&self) -> u16 {
        self.grpc_port
    }

    /// Get a reference to the database.
    pub fn db(&self) -> &GenaiDb {
        &self.state.db
    }

    /// Run the application until stopped.
    ///
    /// This starts both the HTTP health server and the gRPC server concurrently.
    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        // Build minimal HTTP router (health + metrics only)
        let health_state = HealthState {
            db: self.state.db.clone(),
        };

        let http_router = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .with_state(health_state);

        // Build gRPC server
        let genai_service = GenaiGrpcService::new(self.state);

        // gRPC health service
        let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<GenAiServiceServer<GenaiGrpcService>>()
            .await;

        // Reflection service for debugging
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .build_v1()
            .map_err(|e| {
                std::io::Error::other(format!("Failed to build reflection service: {}", e))
            })?;

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(self.grpc_listener);
        let grpc_server = GrpcServer::builder()
            .add_service(grpc_health_service)
            .add_service(reflection_service)
            .add_service(GenAiServiceServer::new(genai_service))
            .serve_with_incoming(incoming);

        // Run both servers concurrently
        tokio::select! {
            result = axum::serve(self.http_listener, http_router) => {
                if let Err(e) = result {
                    tracing::error!("HTTP server error: {}", e);
                    return Err(std::io::Error::other(format!("HTTP server error: {}", e)));
                }
            }
            result = grpc_server => {
                if let Err(e) = result {
                    tracing::error!("gRPC server error: {}", e);
                    return Err(std::io::Error::other(format!("gRPC server error: {}", e)));
                }
            }
        }

        Ok(())
    }
}
