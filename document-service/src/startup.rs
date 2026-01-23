//! Application startup and lifecycle management.
//!
//! This module provides the minimal HTTP server (health/metrics) and gRPC server
//! for the document service. All business logic is exposed via gRPC.

use crate::config::DocumentConfig;
use crate::grpc::{
    proto::{document_service_server::DocumentServiceServer, FILE_DESCRIPTOR_SET},
    CapabilityChecker, DocumentGrpcService,
};
use crate::services::{get_metrics, LocalStorage, MongoDb, Storage};
use crate::workers::{ProcessingJob, WorkerOrchestrator};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use service_core::error::AppError;
use service_core::grpc::interceptors::{metrics_interceptor, trace_context_interceptor};
use service_core::tower::ServiceBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tonic::transport::Server as GrpcServer;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: DocumentConfig,
    pub db: MongoDb,
    pub storage: Arc<dyn Storage>,
    pub job_tx: Option<mpsc::Sender<ProcessingJob>>,
    pub capability_checker: CapabilityChecker,
}

/// State for health check endpoints.
#[derive(Clone)]
struct HealthState {
    db: MongoDb,
}

/// Health check endpoint for Docker/K8s liveness probes.
async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "service": "document-service",
                "version": env!("CARGO_PKG_VERSION")
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "service": "document-service",
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

/// Prometheus metrics endpoint.
async fn metrics_endpoint() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        get_metrics(),
    )
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
    pub async fn build(config: DocumentConfig) -> Result<Self, AppError> {
        // Connect to database
        let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect to MongoDB: {}", e);
                e
            })?;

        db.initialize_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            e
        })?;

        // Initialize storage
        let storage: Arc<dyn Storage> = Arc::new(
            LocalStorage::new(&config.storage.local_path)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to initialize local storage at {}: {}",
                        config.storage.local_path,
                        e
                    );
                    e
                })?,
        );

        // Initialize worker orchestrator
        let (orchestrator, job_tx) =
            WorkerOrchestrator::new(config.worker.clone(), db.clone(), storage.clone());

        // Start worker pool
        tokio::spawn(async move {
            orchestrator.start().await;
        });

        // Initialize capability checker
        let capability_checker =
            CapabilityChecker::new(config.auth.auth_service_endpoint.as_deref())
                .await
                .map_err(|e| {
                    tracing::error!("Failed to initialize capability checker: {}", e);
                    AppError::from(std::io::Error::other(format!(
                        "Capability checker initialization error: {}",
                        e
                    )))
                })?;

        let state = AppState {
            config: config.clone(),
            db: db.clone(),
            storage,
            job_tx: Some(job_tx),
            capability_checker,
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
            "Document service: HTTP on port {}, gRPC on port {}",
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
    pub fn db(&self) -> &MongoDb {
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
            .route("/metrics", get(metrics_endpoint))
            .with_state(health_state);

        // Build gRPC server
        let document_service = DocumentGrpcService::new(self.state);

        // gRPC health service
        let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<DocumentServiceServer<DocumentGrpcService>>()
            .await;

        // Reflection service for debugging
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .build_v1()
            .map_err(|e| {
                std::io::Error::other(format!("Failed to build reflection service: {}", e))
            })?;

        // Apply metering and tracing interceptors
        let layer = ServiceBuilder::new()
            .layer(tonic::service::interceptor(trace_context_interceptor))
            .layer(tonic::service::interceptor(metrics_interceptor))
            .into_inner();

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(self.grpc_listener);
        let grpc_server = GrpcServer::builder()
            .layer(layer)
            .add_service(grpc_health_service)
            .add_service(reflection_service)
            .add_service(DocumentServiceServer::new(document_service))
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
