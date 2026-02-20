//! Auth Service v2 - Main entry point (gRPC-only).

use auth_service::grpc::proto::auth::{
    admin_service_server::AdminServiceServer, assignment_service_server::AssignmentServiceServer,
    audit_service_server::AuditServiceServer, auth_service_server::AuthServiceServer,
    authz_service_server::AuthzServiceServer, invitation_service_server::InvitationServiceServer,
    org_service_server::OrgServiceServer, role_service_server::RoleServiceServer,
    visibility_service_server::VisibilityServiceServer,
};
use service_core::grpc::proto::common::app_registry_service_server::AppRegistryServiceServer;
use auth_service::grpc::{
    AdminServiceImpl, AssignmentServiceImpl, AuditServiceImpl, AuthServiceImpl, AuthzServiceImpl,
    InvitationServiceImpl, OrgServiceImpl, RoleServiceImpl, VisibilityServiceImpl,
};
use auth_service::{config::AuthConfig, db, services, AppState};
use axum::{extract::State, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tonic::transport::Server as GrpcServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration first (before tracing init)
    let config = AuthConfig::from_env()?;

    // Initialize tracing with JSON format for PLG stack
    service_core::observability::init_tracing(&config.service_name, &config.log_level);

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

    // Create email provider via notification-service gRPC
    let email = Arc::new(services::NotificationClient::new(&config.notification.url))
        as Arc<dyn services::EmailProvider>;
    tracing::info!("Email provider initialized (via notification-service)");

    // Create app registry (Redis-backed)
    let app_registry = Arc::new(
        service_core::grpc::AppRegistry::new(&config.redis.url)
            .await
            .expect("Failed to create app registry"),
    );
    let app_registry_svc = service_core::grpc::AppRegistryServiceImpl::new(app_registry.clone());
    tracing::info!("App registry initialized");

    // Build application state
    let state = AppState {
        config: config.clone(),
        db: database,
        email,
        jwt,
        redis,
        app_registry,
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

    let admin_service = AdminServiceImpl::new(state.clone());
    let auth_service = AuthServiceImpl::new(state.clone());
    let authz_service = AuthzServiceImpl::new(state.clone());
    let org_service = OrgServiceImpl::new(state.clone());
    let role_service = RoleServiceImpl::new(state.clone());
    let assignment_service = AssignmentServiceImpl::new(state.clone());
    let invitation_service = InvitationServiceImpl::new(state.clone());
    let visibility_service = VisibilityServiceImpl::new(state.clone());
    let audit_service = AuditServiceImpl::new(state);

    // Create reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(auth_service::grpc::proto::auth::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(
            service_core::grpc::proto::common::APP_REGISTRY_FILE_DESCRIPTOR_SET,
        )
        .build_v1()?;

    // Create gRPC health service
    let (mut health_reporter, grpc_health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AdminServiceServer<AdminServiceImpl>>()
        .await;
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

    tracing::info!("gRPC server listening on {}", grpc_addr);

    let grpc_server = GrpcServer::builder()
        .add_service(grpc_health_service)
        .add_service(reflection_service)
        .add_service(AdminServiceServer::new(admin_service))
        .add_service(AuthServiceServer::new(auth_service))
        .add_service(AuthzServiceServer::new(authz_service))
        .add_service(OrgServiceServer::new(org_service))
        .add_service(RoleServiceServer::new(role_service))
        .add_service(AssignmentServiceServer::new(assignment_service))
        .add_service(InvitationServiceServer::new(invitation_service))
        .add_service(VisibilityServiceServer::new(visibility_service))
        .add_service(AuditServiceServer::new(audit_service))
        .add_service(AppRegistryServiceServer::new(app_registry_svc))
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
