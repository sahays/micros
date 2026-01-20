//! Auth Service v2 - Main entry point (gRPC-only).

use auth_service::grpc::proto::auth::{
    assignment_service_server::AssignmentServiceServer, audit_service_server::AuditServiceServer,
    auth_service_server::AuthServiceServer, authz_service_server::AuthzServiceServer,
    invitation_service_server::InvitationServiceServer, org_service_server::OrgServiceServer,
    role_service_server::RoleServiceServer,
    service_registry_service_server::ServiceRegistryServiceServer,
    visibility_service_server::VisibilityServiceServer,
};
use auth_service::grpc::{
    AssignmentServiceImpl, AuditServiceImpl, AuthServiceImpl, AuthzServiceImpl,
    InvitationServiceImpl, OrgServiceImpl, RoleServiceImpl, ServiceRegistryServiceImpl,
    VisibilityServiceImpl,
};
use auth_service::{config::AuthConfig, db, services, AppState};
use axum::{extract::State, routing::get, Json, Router};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use service_core::grpc::interceptors::trace_context_interceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tonic::transport::Server as GrpcServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration first (before tracing init)
    let config = AuthConfig::from_env()?;

    // Initialize tracing with JSON format for PLG stack
    init_tracing(&config);

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = ?config.environment,
        "Starting auth-service v2 (gRPC)"
    );

    // Create PostgreSQL connection pool
    let pool = db::create_pool(&config.database).await?;
    tracing::info!("PostgreSQL connection pool created");

    // Run migrations
    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    // Create database wrapper
    let database = services::Database::new(pool);

    // Create JWT service
    let jwt = services::JwtService::new(&config.jwt)?;
    tracing::info!("JWT service initialized");

    // Create Redis client
    let redis = Arc::new(services::RedisService::new(&config.redis).await?)
        as Arc<dyn services::TokenBlacklist>;
    tracing::info!("Redis connection established");

    // Create email service
    let email =
        Arc::new(services::EmailService::new(&config.gmail)?) as Arc<dyn services::EmailProvider>;
    tracing::info!("Email service initialized");

    // Build application state
    let state = AppState {
        config: config.clone(),
        db: database,
        email,
        jwt,
        redis,
    };

    // HTTP health endpoint for Docker/K8s probes
    let health_port = config.common.port;
    let health_addr = SocketAddr::from(([0, 0, 0, 0], health_port));
    let health_state = state.clone();

    let health_router = Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .with_state(health_state);

    tracing::info!("HTTP health endpoint listening on {}", health_addr);

    let health_listener = TcpListener::bind(health_addr).await?;
    let health_server = axum::serve(health_listener, health_router.into_make_service());

    // Build gRPC services
    let grpc_port = config.common.port + 1;
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));

    let auth_service = AuthServiceImpl::new(state.clone());
    let authz_service = AuthzServiceImpl::new(state.clone());
    let org_service = OrgServiceImpl::new(state.clone());
    let role_service = RoleServiceImpl::new(state.clone());
    let assignment_service = AssignmentServiceImpl::new(state.clone());
    let invitation_service = InvitationServiceImpl::new(state.clone());
    let visibility_service = VisibilityServiceImpl::new(state.clone());
    let audit_service = AuditServiceImpl::new(state.clone());
    let service_registry_service = ServiceRegistryServiceImpl::new(state);

    // Create reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(auth_service::grpc::proto::auth::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    // Create gRPC health service
    let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AuthServiceServer<AuthServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<AuthzServiceServer<AuthzServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<OrgServiceServer<OrgServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<RoleServiceServer<RoleServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<AssignmentServiceServer<AssignmentServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<InvitationServiceServer<InvitationServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<VisibilityServiceServer<VisibilityServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<AuditServiceServer<AuditServiceImpl>>()
        .await;
    health_reporter
        .set_serving::<ServiceRegistryServiceServer<ServiceRegistryServiceImpl>>()
        .await;

    tracing::info!("gRPC server listening on {}", grpc_addr);

    let grpc_server = GrpcServer::builder()
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(AuthServiceServer::with_interceptor(
            auth_service,
            trace_context_interceptor,
        ))
        .add_service(AuthzServiceServer::with_interceptor(
            authz_service,
            trace_context_interceptor,
        ))
        .add_service(OrgServiceServer::with_interceptor(
            org_service,
            trace_context_interceptor,
        ))
        .add_service(RoleServiceServer::with_interceptor(
            role_service,
            trace_context_interceptor,
        ))
        .add_service(AssignmentServiceServer::with_interceptor(
            assignment_service,
            trace_context_interceptor,
        ))
        .add_service(InvitationServiceServer::with_interceptor(
            invitation_service,
            trace_context_interceptor,
        ))
        .add_service(VisibilityServiceServer::with_interceptor(
            visibility_service,
            trace_context_interceptor,
        ))
        .add_service(AuditServiceServer::with_interceptor(
            audit_service,
            trace_context_interceptor,
        ))
        .add_service(ServiceRegistryServiceServer::with_interceptor(
            service_registry_service,
            trace_context_interceptor,
        ))
        .serve_with_shutdown(grpc_addr, shutdown_signal());

    // Run both servers concurrently
    tokio::select! {
        result = health_server => {
            if let Err(e) = result {
                tracing::error!("HTTP health server error: {}", e);
            }
        }
        result = grpc_server => {
            if let Err(e) = result {
                tracing::error!("gRPC server error: {}", e);
            }
        }
    }

    tracing::info!("Service shutdown complete");
    Ok(())
}

/// Liveness probe - service is running.
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "auth-service",
    }))
}

/// Readiness probe - service is ready to accept requests.
async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Check PostgreSQL connection
    if let Err(e) = state.db.health_check().await {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!("PostgreSQL not ready: {}", e),
        ));
    }

    // Check Redis connection
    if let Err(e) = state.redis.health_check().await {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            format!("Redis not ready: {}", e),
        ));
    }

    Ok(Json(serde_json::json!({
        "status": "ready",
        "service": "auth-service",
        "checks": {
            "postgresql": "up",
            "redis": "up"
        }
    })))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {}", e);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}

/// Initialize tracing with JSON format for PLG stack.
///
/// When OTLP_ENDPOINT is configured, traces are exported to Tempo.
/// Logs are always output as JSON to stdout for Promtail collection.
fn init_tracing(config: &AuthConfig) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));

    // Try to set up OpenTelemetry if OTLP endpoint is configured
    if let Some(ref otlp_endpoint) = config.otlp_endpoint {
        let otlp_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(otlp_endpoint);

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(otlp_exporter)
            .with_trace_config(
                sdktrace::Config::default().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.service_name.clone()),
                    KeyValue::new("service.version", config.service_version.clone()),
                ])),
            )
            .install_batch(runtime::Tokio)
        {
            Ok(tracer) => {
                let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(telemetry)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_file(true)
                            .with_line_number(true)
                            .with_target(true)
                            .json()
                            .flatten_event(true),
                    )
                    .init();
                return;
            }
            Err(e) => {
                eprintln!(
                    "Failed to initialize OTLP tracer (endpoint: {}): {}. Falling back to JSON-only logging.",
                    otlp_endpoint, e
                );
            }
        }
    }

    // Fallback: JSON logging without OpenTelemetry
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .json()
                .flatten_event(true),
        )
        .init();
}
