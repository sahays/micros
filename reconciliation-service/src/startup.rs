//! Application startup and lifecycle management.

use crate::config::ReconciliationConfig;
use crate::grpc::{
    proto::{reconciliation_service_server::ReconciliationServiceServer, FILE_DESCRIPTOR_SET},
    trace_context_interceptor, CapabilityChecker, ReconciliationServiceImpl,
};
use crate::services::{get_metrics, init_metrics, Database};
use axum::{
    extract::State, http::StatusCode, middleware, response::IntoResponse, routing::get, Json,
    Router,
};
use serde_json::json;
use service_core::error::AppError;
use service_core::grpc::LedgerClient;
use service_core::middleware::metrics::metrics_middleware;
use service_core::middleware::tracing::request_id_middleware;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: ReconciliationConfig,
    pub db: Arc<Database>,
    pub capability_checker: Arc<CapabilityChecker>,
    pub ledger_client: Option<Arc<LedgerClient>>,
}

/// State for health check endpoints.
#[derive(Clone)]
struct HealthState {
    db: Arc<Database>,
}

/// Health check endpoint for Docker/K8s liveness probes.
async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => {
            tracing::debug!("Health check passed");
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "service": "reconciliation-service",
                    "version": env!("CARGO_PKG_VERSION")
                })),
            )
        }
        Err(e) => {
            tracing::warn!(error = %e, "Health check failed - database unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "unhealthy",
                    "service": "reconciliation-service",
                    "error": e.to_string()
                })),
            )
        }
    }
}

/// Readiness check endpoint for K8s readiness probes.
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

/// Metrics endpoint for Prometheus scraping.
async fn metrics_handler() -> impl IntoResponse {
    let metrics = get_metrics();
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        metrics,
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
    pub async fn build(config: ReconciliationConfig) -> Result<Self, AppError> {
        Self::build_internal(config, true).await
    }

    /// Build the application without running migrations.
    /// Use this in tests when migrations are already applied by the test harness.
    pub async fn build_without_migrations(config: ReconciliationConfig) -> Result<Self, AppError> {
        Self::build_internal(config, false).await
    }

    async fn build_internal(
        config: ReconciliationConfig,
        run_migrations: bool,
    ) -> Result<Self, AppError> {
        // Initialize metrics
        init_metrics();

        // Connect to database
        let db = Database::new(
            &config.database.url,
            config.database.max_connections,
            config.database.min_connections,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to connect to PostgreSQL");
            e
        })?;

        // Run migrations only if requested
        if run_migrations {
            db.run_migrations().await.map_err(|e| {
                tracing::error!(error = %e, "Failed to run migrations");
                e
            })?;
        }

        let db = Arc::new(db);

        // Create capability checker
        let auth_endpoint = if config.auth.auth_service_endpoint.is_empty() {
            None
        } else {
            Some(config.auth.auth_service_endpoint.as_str())
        };
        let capability_checker =
            Arc::new(CapabilityChecker::new(auth_endpoint).await.map_err(|e| {
                tracing::error!(error = %e, "Failed to create capability checker");
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to create capability checker: {}",
                    e
                ))
            })?);

        // Create ledger client for account validation
        let ledger_client = if !config.ledger_service.url.is_empty() {
            match LedgerClient::connect(&config.ledger_service.url).await {
                Ok(client) => {
                    tracing::info!(endpoint = %config.ledger_service.url, "Connected to ledger-service");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to connect to ledger-service - validation will be skipped");
                    None
                }
            }
        } else {
            tracing::info!("Ledger service URL not configured - validation will be skipped");
            None
        };

        let state = AppState {
            config: config.clone(),
            db,
            capability_checker,
            ledger_client,
        };

        // Bind HTTP listener
        let http_addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
        let http_listener = TcpListener::bind(http_addr).await.map_err(|e| {
            tracing::error!(error = %e, addr = %http_addr, "Failed to bind HTTP listener");
            AppError::from(e)
        })?;
        let http_port = http_listener.local_addr()?.port();

        // Bind gRPC listener (port + 1)
        let grpc_addr = SocketAddr::from(([0, 0, 0, 0], config.common.port + 1));
        let grpc_listener = TcpListener::bind(grpc_addr).await.map_err(|e| {
            tracing::error!(error = %e, addr = %grpc_addr, "Failed to bind gRPC listener");
            AppError::from(e)
        })?;
        let grpc_port = grpc_listener.local_addr()?.port();

        tracing::info!(
            http_port = http_port,
            grpc_port = grpc_port,
            "Reconciliation service listeners bound"
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
    pub fn db(&self) -> &Database {
        &self.state.db
    }

    /// Run the application until stopped.
    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        // Build minimal HTTP router (health + metrics)
        let health_state = HealthState {
            db: self.state.db.clone(),
        };

        let http_router = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .route("/metrics", get(metrics_handler))
            .layer(TraceLayer::new_for_http())
            .layer(middleware::from_fn(metrics_middleware))
            .layer(middleware::from_fn(request_id_middleware))
            .with_state(health_state);

        // Build gRPC server
        let reconciliation_service = ReconciliationServiceImpl::new(
            self.state.db.clone(),
            self.state.capability_checker.clone(),
            self.state.ledger_client.clone(),
        );

        // gRPC health service
        let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<ReconciliationServiceServer<ReconciliationServiceImpl>>()
            .await;

        // Reflection service for debugging
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .build_v1()
            .map_err(|e| {
                std::io::Error::other(format!("Failed to build reflection service: {}", e))
            })?;

        // gRPC trace layer for observability
        let grpc_trace_layer = TraceLayer::new_for_grpc()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::DEBUG));

        // Create reconciliation service with trace context interceptor for W3C trace propagation
        let reconciliation_service_with_interceptor = ReconciliationServiceServer::with_interceptor(
            reconciliation_service,
            trace_context_interceptor,
        );

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(self.grpc_listener);
        let grpc_server = GrpcServer::builder()
            .layer(grpc_trace_layer)
            .add_service(grpc_health_service)
            .add_service(reflection_service)
            .add_service(reconciliation_service_with_interceptor)
            .serve_with_incoming(incoming);

        tracing::info!(
            service = "reconciliation-service",
            version = env!("CARGO_PKG_VERSION"),
            http_port = self.http_port,
            grpc_port = self.grpc_port,
            "Service ready to accept connections"
        );

        // Run both servers concurrently
        tokio::select! {
            result = axum::serve(self.http_listener, http_router) => {
                if let Err(e) = result {
                    tracing::error!(error = %e, "HTTP server error");
                    return Err(std::io::Error::other(format!("HTTP server error: {}", e)));
                }
            }
            result = grpc_server => {
                if let Err(e) = result {
                    tracing::error!(error = %e, "gRPC server error");
                    return Err(std::io::Error::other(format!("gRPC server error: {}", e)));
                }
            }
        }

        Ok(())
    }
}
