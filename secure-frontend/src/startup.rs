use axum::{middleware::from_fn, routing::get, Router};
use service_core::middleware::{metrics::metrics_middleware, tracing::request_id_middleware};
use std::sync::Arc;
use time::Duration;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use crate::handlers::{
    admin::{admin_dashboard_handler, service_list_fragment, user_list_fragment},
    app::{health_check, index},
    auth::{login_handler, login_page, logout_handler, register_handler, register_page},
    user::dashboard_handler,
};
use crate::middleware::auth::auth_middleware;
use crate::services::auth_client::AuthClient;

pub fn build_router(auth_client: Arc<AuthClient>) -> Router {
    // Session setup
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/metrics", get(crate::handlers::metrics::metrics))
        .route("/login", get(login_page).post(login_handler))
        .route("/register", get(register_page).post(register_handler))
        .route("/logout", get(logout_handler))
        .route(
            "/dashboard",
            get(dashboard_handler).layer(axum::middleware::from_fn_with_state(
                auth_client.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/admin",
            get(admin_dashboard_handler).layer(axum::middleware::from_fn_with_state(
                auth_client.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/admin/services/list",
            get(service_list_fragment).layer(axum::middleware::from_fn_with_state(
                auth_client.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/admin/users/list",
            get(user_list_fragment).layer(axum::middleware::from_fn_with_state(
                auth_client.clone(),
                auth_middleware,
            )),
        )
        .nest_service("/static", ServeDir::new("secure-frontend/static"))
        .layer(session_layer)
        .layer(from_fn(metrics_middleware))
        // Add tracing layer
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request
                    .headers()
                    .get("x-request-id")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("-");

                tracing::info_span!(
                    "http_request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            }),
        )
        // Add tracing middleware for request_id
        .layer(from_fn(request_id_middleware))
        .with_state(auth_client)
}
