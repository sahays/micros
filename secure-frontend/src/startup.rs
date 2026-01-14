use axum::http::header::{HeaderValue, CACHE_CONTROL};
use axum::{middleware::from_fn, routing::get, Router};
use service_core::middleware::{metrics::metrics_middleware, tracing::request_id_middleware};
use time::Duration;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use crate::handlers::{
    admin::{admin_dashboard_handler, service_list_fragment, user_list_fragment},
    app::{health_check, index},
    auth::{
        google_oauth_callback, google_oauth_redirect, login_handler, login_page, logout_handler,
        register_handler, register_page,
    },
    documents::list_documents_page,
    download::{download_document, download_with_signature, generate_signed_url},
    upload::{upload_handler, upload_page},
    user::dashboard_handler,
};
use crate::middleware::auth::auth_middleware;
use crate::AppState;

pub fn build_router(app_state: AppState) -> Router {
    // Session setup
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    // Static file service with aggressive caching (1 year)
    let static_service = ServeDir::new("secure-frontend/static")
        .precompressed_gzip()
        .precompressed_br();

    // API routes with full middleware stack
    let api_routes = Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/metrics", get(crate::handlers::metrics::metrics))
        .route("/login", get(login_page).post(login_handler))
        .route("/register", get(register_page).post(register_handler))
        .route("/logout", get(logout_handler))
        .route("/auth/google", get(google_oauth_redirect))
        .route("/auth/google/callback", get(google_oauth_callback))
        .route(
            "/dashboard",
            get(dashboard_handler).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/documents",
            get(list_documents_page).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/documents/upload",
            get(upload_page)
                .post(upload_handler)
                .layer(axum::middleware::from_fn_with_state(
                    app_state.clone(),
                    auth_middleware,
                )),
        )
        .route(
            "/documents/:id/download",
            get(download_document).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/documents/:id/share",
            get(generate_signed_url).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route("/share/:id", get(download_with_signature))
        .route(
            "/admin",
            get(admin_dashboard_handler).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/admin/services/list",
            get(service_list_fragment).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/admin/users/list",
            get(user_list_fragment).layer(axum::middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            )),
        )
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
        .with_state(app_state);

    // Static files with cache headers (bypass heavy middleware)
    let static_routes = Router::new().nest_service("/static", static_service).layer(
        SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ),
    );

    // Merge routes - static files bypass session/metrics/tracing middleware
    Router::new().merge(api_routes).merge(static_routes)
}
