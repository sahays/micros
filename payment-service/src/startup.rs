//! Application startup and lifecycle management.
//!
//! This module provides the minimal HTTP server (health/metrics) and gRPC server
//! for the payment service. All business logic is exposed via gRPC.

use crate::config::Config;
use crate::grpc::{
    proto::{payment_service_server::PaymentServiceServer, FILE_DESCRIPTOR_SET},
    CapabilityChecker, PaymentGrpcService,
};
use crate::services::{get_metrics, PaymentRepository, RazorpayClient};
use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use mongodb::{options::ClientOptions, Client};
use secrecy::ExposeSecret;
use serde_json::json;
use service_core::error::AppError;
use service_core::grpc::interceptors::{metrics_interceptor, trace_context_interceptor};
use service_core::middleware::signature::SignatureConfig;
use service_core::tower::ServiceBuilder;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub db: mongodb::Database,
    pub redis: redis::Client,
    pub config: Config,
    pub signature_config: SignatureConfig,
    pub repository: PaymentRepository,
    pub razorpay: RazorpayClient,
    pub capability_checker: CapabilityChecker,
}

/// Health check endpoint for Docker/K8s liveness probes.
async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "payment-service",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}

/// Readiness check endpoint for K8s readiness probes.
async fn readiness_check() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ready" })))
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
    pub async fn build(config: Config) -> Result<Self, AppError> {
        // Connect to MongoDB
        let mut client_options = ClientOptions::parse(config.database.url.expose_secret())
            .await
            .map_err(|e| {
                tracing::error!("Failed to parse MongoDB connection string: {}", e);
                AppError::DatabaseError(e.into())
            })?;
        client_options.app_name = Some("payment-service".to_string());

        let client = Client::with_options(client_options).map_err(|e| {
            tracing::error!("Failed to create MongoDB client: {}", e);
            AppError::DatabaseError(e.into())
        })?;
        let db = client.database(&config.database.db_name);

        // Connect to Redis
        let redis =
            redis::Client::open(config.redis.url.expose_secret().as_str()).map_err(|e| {
                tracing::error!("Failed to connect to Redis: {}", e);
                AppError::InternalError(e.into())
            })?;

        let signature_config = SignatureConfig {
            require_signatures: config.signature.enabled,
            excluded_paths: vec!["/health".to_string(), "/ready".to_string()],
        };

        let repository = PaymentRepository::new(&db);

        // Initialize indexes for tenant-scoped queries
        repository.init_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            AppError::DatabaseError(e)
        })?;

        // Initialize Razorpay client
        let razorpay = RazorpayClient::new(config.razorpay.clone());
        if razorpay.is_configured() {
            tracing::info!("Razorpay client initialized");
        } else {
            tracing::warn!(
                "Razorpay credentials not configured - payment features will be limited"
            );
        }

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
            db,
            redis,
            config: config.clone(),
            signature_config,
            repository,
            razorpay,
            capability_checker,
        };

        // Bind HTTP listener (port 0 = random port for testing)
        let http_addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
        let http_listener = TcpListener::bind(http_addr).await.map_err(|e| {
            tracing::error!("Failed to bind HTTP listener to {}: {}", http_addr, e);
            AppError::from(e)
        })?;
        let http_port = http_listener.local_addr()?.port();

        // Bind gRPC listener
        let grpc_addr = SocketAddr::from(([0, 0, 0, 0], config.server.grpc_port));
        let grpc_listener = TcpListener::bind(grpc_addr).await.map_err(|e| {
            tracing::error!("Failed to bind gRPC listener to {}: {}", grpc_addr, e);
            AppError::from(e)
        })?;
        let grpc_port = grpc_listener.local_addr()?.port();

        tracing::info!(
            "Payment service: HTTP on port {}, gRPC on port {}",
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
    pub fn db(&self) -> &mongodb::Database {
        &self.state.db
    }

    /// Get the application state for sharing with gRPC service.
    pub fn state(&self) -> AppState {
        self.state.clone()
    }

    /// Run the application until stopped.
    ///
    /// This starts both the HTTP health server and the gRPC server concurrently.
    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        // Build minimal HTTP router (health + metrics)
        let http_router = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .route("/metrics", get(metrics_endpoint));

        // Build gRPC server
        let payment_service = PaymentGrpcService::new(self.state);

        // gRPC health service
        let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<PaymentServiceServer<PaymentGrpcService>>()
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
            .add_service(PaymentServiceServer::new(payment_service))
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
