//! Auth Service v2
//!
//! PostgreSQL-backed authentication and authorization service with:
//! - Capability-based authorization
//! - Org node hierarchy with closure table
//! - Time-bounded immutable assignments
//! - Know-Your-Service (KYS) registry

pub mod config;
pub mod db;
pub mod handlers;
pub mod models;
pub mod services;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::config::AuthConfig;
use crate::handlers::{assignment, auth, authz, context, org, role, service};
use crate::services::{Database, EmailProvider, JwtService, TokenBlacklist};
use service_core::error::AppError;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: AuthConfig,
    pub db: Database,
    pub email: Arc<dyn EmailProvider>,
    pub jwt: JwtService,
    pub redis: Arc<dyn TokenBlacklist>,
}

/// Build the application router.
pub async fn build_router(state: AppState) -> Result<Router, AppError> {
    // Auth routes
    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        .route("/logout", post(auth::logout))
        .route("/context", get(context::get_auth_context))
        .route("/check", get(context::check_capability));

    // Org routes
    let org_routes = Router::new()
        .route("/", post(org::create_org_node))
        .route("/{org_node_id}", get(org::get_org_node))
        .route(
            "/{org_node_id}/descendants",
            get(org::get_org_node_descendants),
        );

    // Tenant-scoped org routes
    let tenant_org_routes = Router::new()
        .route("/", get(org::list_tenant_org_nodes))
        .route("/tree", get(org::get_tenant_org_tree));

    // Role routes
    let role_routes = Router::new()
        .route("/", post(role::create_role))
        .route("/{role_id}", get(role::get_role))
        .route("/{role_id}/capabilities", get(role::get_role_capabilities))
        .route("/{role_id}/capabilities", post(role::assign_capability));

    // Tenant-scoped role routes
    let tenant_role_routes = Router::new().route("/", get(role::list_tenant_roles));

    // Capability routes
    let capability_routes = Router::new()
        .route("/", get(role::list_capabilities))
        .route("/{cap_key}", get(role::get_capability));

    // Assignment routes
    let assignment_routes = Router::new()
        .route("/", post(assignment::create_assignment))
        .route("/{assignment_id}/end", post(assignment::end_assignment));

    // User assignment routes
    let user_assignment_routes = Router::new().route("/", get(assignment::list_user_assignments));

    // AuthZ routes
    let authz_routes = Router::new()
        .route("/evaluate", post(authz::evaluate))
        .route("/batch-evaluate", post(authz::batch_evaluate));

    // Service routes (KYS - Know Your Service)
    let service_routes = Router::new()
        .route("/", post(service::register_service))
        .route("/token", post(service::get_service_token))
        .route("/{svc_key}", get(service::get_service))
        .route("/{svc_key}/rotate-secret", post(service::rotate_secret))
        .route(
            "/{svc_key}/permissions",
            get(service::get_service_permissions),
        )
        .route("/{svc_key}/permissions", post(service::grant_permission));

    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/auth", auth_routes)
        .nest("/orgs", org_routes)
        .nest("/tenants/{tenant_id}/orgs", tenant_org_routes)
        .nest("/roles", role_routes)
        .nest("/tenants/{tenant_id}/roles", tenant_role_routes)
        .nest("/capabilities", capability_routes)
        .nest("/assignments", assignment_routes)
        .nest("/users/{user_id}/assignments", user_assignment_routes)
        .nest("/authz", authz_routes)
        .nest("/services", service_routes)
        .with_state(state.clone())
        .layer(CorsLayer::permissive());

    Ok(app)
}

/// Health check endpoint.
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check PostgreSQL connection
    state.db.health_check().await?;

    // Check Redis connection
    state.redis.health_check().await.map_err(|e| {
        tracing::error!(error = %e, "Redis health check failed");
        AppError::InternalError(e)
    })?;

    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": state.config.service_name,
        "version": state.config.service_version,
        "checks": {
            "postgresql": "up",
            "redis": "up"
        }
    })))
}
