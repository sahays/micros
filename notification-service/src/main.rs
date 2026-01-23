use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use notification_service::config::NotificationConfig;
use notification_service::grpc::{
    proto::{notification_service_server::NotificationServiceServer, FILE_DESCRIPTOR_SET},
    CapabilityChecker, NotificationGrpcService,
};
use notification_service::services::{
    get_metrics, init_metrics, EmailProvider, FcmProvider, MockEmailProvider, MockPushProvider,
    MockSmsProvider, Msg91Provider, NotificationDb, PushProvider, SmsProvider, SmtpProvider,
};
use notification_service::startup::AppState;
use serde_json::json;
use service_core::grpc::interceptors::{metrics_interceptor, trace_context_interceptor};
use service_core::observability::init_tracing;
use service_core::tower::ServiceBuilder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tonic::transport::Server as GrpcServer;

#[derive(Clone)]
struct HealthState {
    db: NotificationDb,
}

async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
    // Check database connectivity
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

async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
    match state.db.health_check().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn metrics_endpoint() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        get_metrics(),
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

    tracing::info!("Shutdown signal received");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://tempo:4317".to_string());
    init_tracing("notification-service", "info", &otlp_endpoint);

    // Initialize metrics
    init_metrics();

    let config = NotificationConfig::load().map_err(|e| {
        tracing::error!("Failed to load configuration: {}", e);
        std::io::Error::other(format!("Configuration error: {}", e))
    })?;

    // Connect to database
    let db = NotificationDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to MongoDB: {}", e);
            std::io::Error::other(format!("Database connection error: {}", e))
        })?;

    db.initialize_indexes().await.map_err(|e| {
        tracing::error!("Failed to initialize database indexes: {}", e);
        std::io::Error::other(format!("Database initialization error: {}", e))
    })?;

    // Initialize providers
    let email_provider: Arc<dyn EmailProvider> = if config.smtp.enabled {
        match SmtpProvider::new(config.smtp.clone()) {
            Ok(provider) => {
                tracing::info!("SMTP email provider initialized");
                Arc::new(provider)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize SMTP provider: {}. Using mock.", e);
                Arc::new(MockEmailProvider::new(true))
            }
        }
    } else {
        tracing::info!("SMTP provider disabled, using mock email provider");
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
    let capability_checker = CapabilityChecker::new(config.auth.auth_service_endpoint.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize capability checker: {}", e);
            std::io::Error::other(format!("Capability checker initialization error: {}", e))
        })?;

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email_provider,
        sms_provider,
        push_provider,
        capability_checker,
    };

    let health_state = HealthState { db };

    // HTTP health endpoint for Docker/K8s probes
    let health_router = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/metrics", get(metrics_endpoint))
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
    let notification_service = NotificationGrpcService::new(state);

    // gRPC health service
    let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<NotificationServiceServer<NotificationGrpcService>>()
        .await;

    // Reflection service for debugging
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|e| std::io::Error::other(format!("Failed to build reflection service: {}", e)))?;

    tracing::info!("gRPC server listening on port {}", grpc_port);

    // Apply metering and tracing interceptors
    let layer = ServiceBuilder::new()
        .layer(tonic::service::interceptor(trace_context_interceptor))
        .layer(tonic::service::interceptor(metrics_interceptor))
        .into_inner();

    // Build gRPC server
    let grpc_server = GrpcServer::builder()
        .layer(layer)
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(NotificationServiceServer::new(notification_service))
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
