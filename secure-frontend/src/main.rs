use axum::{routing::get, Router};
use dotenvy::dotenv;
use secure_frontend::config::get_configuration;
use secure_frontend::handlers::{
    admin::{admin_dashboard_handler, service_list_fragment, user_list_fragment},
    app::{health_check, index},
    auth::{login_handler, login_page, logout_handler, register_handler, register_page},
    user::dashboard_handler,
};
use secure_frontend::middleware::auth::auth_middleware;
use secure_frontend::services::auth_client::AuthClient;
use secure_frontend::utils::init_tracing;
use std::sync::Arc;
use time::Duration;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    init_tracing();
    secure_frontend::services::metrics::init_metrics();

    let configuration = get_configuration().expect("Failed to read configuration.");

    let auth_client = Arc::new(AuthClient::new(configuration.auth_service.clone()));

    // Session setup
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/metrics", get(secure_frontend::handlers::metrics::metrics))
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
        .layer(axum::middleware::from_fn(
            secure_frontend::middleware::metrics::metrics_middleware,
        ))
        .with_state(auth_client);

    let address = format!(
        "{}:{}",
        configuration.server.host, configuration.server.port
    );
    let listener = tokio::net::TcpListener::bind(&address).await?;

    info!("Starting secure-frontend on {}", address);
    axum::serve(listener, app).await?;

    Ok(())
}
