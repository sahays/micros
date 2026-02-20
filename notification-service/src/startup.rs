//! Application startup and lifecycle management.
//!
//! This module provides the minimal HTTP server (health/metrics) and gRPC server
//! for the notification service. All business logic is exposed via gRPC.

use crate::config::NotificationConfig;
use crate::grpc::{
    proto::{notification_service_server::NotificationServiceServer, FILE_DESCRIPTOR_SET},
    CapabilityChecker, NotificationGrpcService,
};
use service_core::grpc::proto::common::app_registry_service_server::AppRegistryServiceServer;
use crate::services::{
    EmailProvider, FcmProvider, GmailApiProvider, MockEmailProvider, MockPushProvider,
    MockSmsProvider, Msg91Provider, NotificationDb, PushProvider, SmsProvider,
};
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
    pub config: NotificationConfig,
    pub db: NotificationDb,
    pub email_provider: Arc<dyn EmailProvider>,
    pub sms_provider: Arc<dyn SmsProvider>,
    pub push_provider: Arc<dyn PushProvider>,
    pub capability_checker: CapabilityChecker,
}

/// State for health check endpoints.
#[derive(Clone)]
struct HealthState {
    db: NotificationDb,
}

/// Health check endpoint for Docker/K8s liveness probes.
async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "service": "notification-service",
                "version": env!("CARGO_PKG_VERSION")
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "service": "notification-service",
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
    pub async fn build(config: NotificationConfig) -> Result<Self, AppError> {
        // Connect to database
        let db = NotificationDb::connect(&config.mongodb.uri, &config.mongodb.database)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect to MongoDB: {}", e);
                e
            })?;

        db.initialize_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            e
        })?;

        // Initialize providers
        let is_prod =
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()) == "prod";
        let email_provider: Arc<dyn EmailProvider> = if config.gmail.enabled {
            match GmailApiProvider::new(config.gmail.clone()) {
                Ok(provider) => {
                    tracing::info!("Gmail API email provider initialized");
                    Arc::new(provider)
                }
                Err(e) => {
                    if is_prod {
                        tracing::error!(
                            "Failed to initialize Gmail API provider in production: {}",
                            e
                        );
                        return Err(AppError::ConfigError(anyhow::anyhow!(
                            "Gmail API is enabled but failed to initialize: {}. Refusing to start in production with mock provider.",
                            e
                        )));
                    }
                    tracing::warn!(
                        "Failed to initialize Gmail API provider: {}. Using mock in dev.",
                        e
                    );
                    Arc::new(MockEmailProvider::new(true))
                }
            }
        } else {
            if is_prod {
                tracing::warn!("Gmail API provider is DISABLED in production — emails will not be sent");
            } else {
                tracing::info!("Gmail API provider disabled, using mock email provider");
            }
            Arc::new(MockEmailProvider::new(true))
        };

        let sms_provider: Arc<dyn SmsProvider> = if config.msg91.enabled {
            tracing::info!("Msg91 SMS provider initialized");
            Arc::new(Msg91Provider::new(config.msg91.clone()))
        } else {
            tracing::info!("Msg91 provider disabled, using mock SMS provider");
            Arc::new(MockSmsProvider::new(true))
        };

        let push_provider: Arc<dyn PushProvider> = if config.fcm.enabled {
            tracing::info!("FCM push provider initialized");
            Arc::new(FcmProvider::new(config.fcm.clone()))
        } else {
            tracing::info!("FCM provider disabled, using mock push provider");
            Arc::new(MockPushProvider::new(true))
        };

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
            db,
            email_provider,
            sms_provider,
            push_provider,
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
            "Notification service: HTTP on port {}, gRPC on port {}",
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
    pub fn db(&self) -> &NotificationDb {
        &self.state.db
    }

    /// Run the application until stopped.
    ///
    /// This starts both the HTTP health server and the gRPC server concurrently.
    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        // Build minimal HTTP router (health only)
        let health_state = HealthState {
            db: self.state.db.clone(),
        };

        let http_router = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .with_state(health_state);

        // Build gRPC server
        let notification_service = NotificationGrpcService::new(self.state);

        // App registry (Redis-backed)
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let app_registry = std::sync::Arc::new(
            service_core::grpc::AppRegistry::new(&redis_url)
                .await
                .map_err(|e| {
                    std::io::Error::other(format!("Failed to create app registry: {}", e))
                })?,
        );
        let app_registry_svc = service_core::grpc::AppRegistryServiceImpl::new(app_registry);

        // gRPC health service
        let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<NotificationServiceServer<NotificationGrpcService>>()
            .await;

        // Reflection service for debugging
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(
                service_core::grpc::proto::common::APP_REGISTRY_FILE_DESCRIPTOR_SET,
            )
            .build_v1()
            .map_err(|e| {
                std::io::Error::other(format!("Failed to build reflection service: {}", e))
            })?;

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(self.grpc_listener);
        let grpc_server = GrpcServer::builder()
            .add_service(grpc_health_service)
            .add_service(reflection_service)
            .add_service(NotificationServiceServer::new(notification_service))
            .add_service(AppRegistryServiceServer::new(app_registry_svc))
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
