pub mod config;
pub mod dtos;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

use service_core::axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};
use service_core::middleware::{
    bot_detection::bot_detection_middleware, metrics::metrics_middleware,
    rate_limit::ip_rate_limit_middleware, security_headers::security_headers_middleware,
    signature::signature_validation_middleware, tracing::request_id_middleware,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AuthConfig;
use crate::services::{EmailProvider, JwtService, MongoDb};
use service_core::error::AppError;
use std::sync::Arc;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        handlers::well_known::jwks,
        handlers::auth::registration::register,
        handlers::auth::registration::verify_email,
        handlers::auth::session::login,
        handlers::auth::session::logout,
        handlers::auth::session::refresh,
        handlers::auth::session::introspect,
        handlers::auth::password::request_password_reset,
        handlers::auth::password::confirm_password_reset,
        handlers::app::app_token,
        handlers::user::get_me,
        handlers::user::update_me,
        handlers::user::change_password,
        handlers::admin::clients::create_client,
        handlers::admin::clients::rotate_client_secret,
        handlers::admin::clients::revoke_client,
        handlers::admin::service_accounts::create_service_account,
        handlers::admin::service_accounts::rotate_service_key,
        handlers::admin::service_accounts::revoke_service_account,
        handlers::admin::service_accounts::get_service_audit_log,
    ),
    components(
        schemas(
            dtos::auth::RegisterRequest,
            dtos::auth::RegisterResponse,
            dtos::ErrorResponse,
            dtos::auth::VerifyResponse,
            dtos::auth::LoginRequest,
            dtos::auth::LogoutRequest,
            dtos::auth::RefreshRequest,
            dtos::auth::IntrospectRequest,
            dtos::auth::IntrospectResponse,
            dtos::auth::PasswordResetRequest,
            dtos::auth::PasswordResetConfirm,
            handlers::app::AppTokenRequest,
            services::TokenResponse,
            handlers::user::ChangePasswordRequest,
            handlers::user::UpdateUserRequest,
            dtos::admin::CreateClientRequest,
            dtos::admin::CreateClientResponse,
            dtos::admin::RotateSecretResponse,
            dtos::admin::CreateServiceAccountRequest,
            dtos::admin::CreateServiceAccountResponse,
            dtos::admin::RotateServiceKeyResponse,
            models::User,
            models::SanitizedUser,
            models::Client,
            models::ClientType,
            models::ServiceAccount,
            models::AuditLog,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "User authentication and token management"),
        (name = "Service Authentication", description = "Service-to-service authentication"),
        (name = "User", description = "User profile management"),
        (name = "Admin", description = "Administrative operations"),
        (name = "Well-Known", description = "Public service metadata"),
        (name = "Observability", description = "Service health and monitoring"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "admin_api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-admin-api-key"))),
            );
            components.add_security_scheme(
                "app_token",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-app-token"))),
            );
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: AuthConfig,
    pub db: MongoDb,
    pub email: Arc<dyn EmailProvider>,
    pub jwt: JwtService,
    pub auth_service: crate::services::AuthService,
    pub admin_service: crate::services::admin::AdminService,
    pub redis: Arc<dyn crate::services::TokenBlacklist>,
    pub login_rate_limiter: service_core::middleware::rate_limit::IpRateLimiter,
    pub register_rate_limiter: service_core::middleware::rate_limit::IpRateLimiter,
    pub password_reset_rate_limiter: service_core::middleware::rate_limit::IpRateLimiter,
    pub app_token_rate_limiter: service_core::middleware::rate_limit::IpRateLimiter,
    pub client_rate_limiter: service_core::middleware::rate_limit::ClientRateLimiter,
    pub ip_rate_limiter: service_core::middleware::rate_limit::IpRateLimiter,
}

impl AsRef<service_core::middleware::signature::SignatureConfig> for AppState {
    fn as_ref(&self) -> &service_core::middleware::signature::SignatureConfig {
        // Map AuthConfig to SignatureConfig (or just store SignatureConfig in AppState)
        // For now, we'll implement it by returning a reference to something we can store
        // But since AppState is already defined, let's just create one on the fly for simplicity
        // in this implementation or add it to AppState.
        // Better: add it to AppState during construction in main.rs.
        &self.config.security.signature_config
    }
}

#[service_core::axum::async_trait]
impl service_core::middleware::signature::SignatureStore for AppState {
    async fn validate_nonce(&self, nonce: &str) -> Result<bool, AppError> {
        let nonce_key = format!("nonce:{}", nonce);
        let val = self.redis.get_cache(&nonce_key).await.map_err(|e| {
            tracing::error!("Failed to check nonce {} in Redis: {}", nonce, e);
            AppError::InternalError(anyhow::anyhow!("Failed to validate nonce: {}", e))
        })?;
        if val.is_some() {
            return Ok(false);
        }
        self.redis
            .set_cache(&nonce_key, "1", 120)
            .await
            .map_err(|e| {
                tracing::error!("Failed to store nonce {} in Redis: {}", nonce, e);
                AppError::InternalError(anyhow::anyhow!("Failed to store nonce: {}", e))
            })?;
        Ok(true)
    }

    async fn get_signing_secret(&self, client_id: &str) -> Result<Option<String>, AppError> {
        let client: Option<crate::models::Client> = self
            .db
            .clients()
            .find_one(
                service_core::mongodb::bson::doc! { "client_id": client_id },
                None,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to lookup client {} signing secret: {}",
                    client_id,
                    e
                );
                AppError::from(e)
            })?;
        Ok(client.map(|c| c.signing_secret))
    }
}

pub async fn build_router(state: AppState) -> Result<Router, AppError> {
    // Admin routes
    let admin_routes = Router::new()
        .route("/auth/admin/clients", post(handlers::admin::create_client))
        .route(
            "/auth/admin/clients/:client_id/rotate",
            post(handlers::admin::rotate_client_secret),
        )
        .route(
            "/auth/admin/clients/:client_id",
            service_core::axum::routing::delete(handlers::admin::revoke_client),
        )
        .route(
            "/auth/admin/services",
            post(handlers::admin::create_service_account),
        )
        .route(
            "/auth/admin/services/:service_id/rotate",
            post(handlers::admin::rotate_service_key),
        )
        .route(
            "/auth/admin/services/:service_id",
            service_core::axum::routing::delete(handlers::admin::revoke_service_account),
        )
        .route(
            "/auth/admin/services/:service_id/audit-log",
            get(handlers::admin::get_service_audit_log),
        )
        .layer(from_fn_with_state(
            state.clone(),
            middleware::admin_auth_middleware,
        ));

    // Create login route with rate limiting
    let login_limiter = state.login_rate_limiter.clone();
    let login_route = Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .layer(from_fn_with_state(login_limiter, ip_rate_limit_middleware));

    // Create register route with rate limiting
    let register_limiter = state.register_rate_limiter.clone();
    let register_route = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .layer(from_fn_with_state(
            register_limiter,
            ip_rate_limit_middleware,
        ));

    // Create password reset request route with rate limiting
    let reset_request_limiter = state.password_reset_rate_limiter.clone();
    let reset_request_route = Router::new()
        .route(
            "/auth/password-reset/request",
            post(handlers::auth::request_password_reset),
        )
        .layer(from_fn_with_state(
            reset_request_limiter,
            ip_rate_limit_middleware,
        ));

    // Create app token route with rate limiting
    let app_token_limiter = state.app_token_rate_limiter.clone();
    let app_token_route = Router::new()
        .route("/auth/app/token", post(handlers::app::app_token))
        .layer(from_fn_with_state(
            app_token_limiter,
            ip_rate_limit_middleware,
        ));

    // Create global IP rate limiter
    let ip_limiter = state.ip_rate_limiter.clone();

    // Configure Swagger UI
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(handlers::metrics::metrics))
        .route("/.well-known/jwks.json", get(handlers::well_known::jwks));

    // Only add Swagger UI if enabled in config
    let swagger_enabled = match state.config.environment {
        crate::config::Environment::Dev => true,
        crate::config::Environment::Prod => match state.config.swagger.enabled {
            crate::config::SwaggerMode::Public | crate::config::SwaggerMode::Authenticated => true,
            crate::config::SwaggerMode::Disabled => false,
        },
    };

    if swagger_enabled {
        app =
            app.merge(SwaggerUi::new("/docs").url("/.well-known/openapi.json", ApiDoc::openapi()));
    } else {
        // If Swagger UI is disabled, still provide the OpenAPI JSON for programmatic access
        app = app.route(
            "/.well-known/openapi.json",
            get(|| async { service_core::axum::Json(ApiDoc::openapi()) }),
        );
    }

    let app = app
        // Authentication routes
        .route("/auth/verify", get(handlers::auth::verify_email))
        .route("/auth/introspect", post(handlers::auth::introspect))
        .route("/auth/google", get(handlers::auth::google_login))
        .route(
            "/auth/google/callback",
            get(handlers::auth::google_callback),
        )
        .merge(app_token_route)
        .merge(login_route)
        .merge(register_route)
        .merge(reset_request_route)
        .merge(admin_routes)
        .route(
            "/auth/password-reset/confirm",
            post(handlers::auth::confirm_password_reset),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .merge(
            Router::new()
                .route("/auth/logout", post(handlers::auth::logout))
                .route(
                    "/users/me",
                    get(handlers::user::get_me).patch(handlers::user::update_me),
                )
                .route("/users/me/password", post(handlers::user::change_password))
                .layer(from_fn_with_state(
                    state.clone(),
                    middleware::auth_middleware,
                )),
        )
        .with_state(state.clone())
        // Global IP rate limiting
        .layer(from_fn_with_state(ip_limiter, ip_rate_limit_middleware))
        // Signature validation
        .layer(from_fn_with_state(
            state.clone(),
            signature_validation_middleware::<AppState>,
        ))
        // Add metrics middleware
        .layer(from_fn(metrics_middleware))
        // Add tracing layer
        .layer(TraceLayer::new_for_http().make_span_with(
            |request: &service_core::axum::http::Request<_>| {
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
            },
        ))
        // Add tracing middleware for request_id
        .layer(from_fn(request_id_middleware))
        // Add security headers middleware
        .layer(from_fn(security_headers_middleware))
        // Add bot detection middleware
        .layer(from_fn(bot_detection_middleware))
        // Add CORS layer
        .layer(
            CorsLayer::new()
                .allow_origin(
                    state
                        .config
                        .security
                        .allowed_origins
                        .iter()
                        .map(|o| {
                            o.parse::<service_core::axum::http::HeaderValue>()
                                .unwrap_or_else(|e| {
                                    tracing::error!(
                                        "Invalid CORS origin '{}': {}. Using fallback.",
                                        o,
                                        e
                                    );
                                    service_core::axum::http::HeaderValue::from_static("*")
                                })
                        })
                        .collect::<Vec<service_core::axum::http::HeaderValue>>(),
                )
                .allow_methods([
                    service_core::axum::http::Method::GET,
                    service_core::axum::http::Method::POST,
                    service_core::axum::http::Method::PATCH,
                    service_core::axum::http::Method::DELETE,
                    service_core::axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    service_core::axum::http::header::AUTHORIZATION,
                    service_core::axum::http::header::CONTENT_TYPE,
                    service_core::axum::http::header::HeaderName::from_static("x-admin-api-key"),
                    service_core::axum::http::header::HeaderName::from_static("x-app-token"),
                    service_core::axum::http::header::HeaderName::from_static("x-client-id"),
                    service_core::axum::http::header::HeaderName::from_static("x-timestamp"),
                    service_core::axum::http::header::HeaderName::from_static("x-nonce"),
                    service_core::axum::http::header::HeaderName::from_static("x-signature"),
                ]),
        );

    Ok(app)
}

/// Service health check
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy"),
        (status = 503, description = "Service is unhealthy")
    ),
    tag = "Observability"
)]
pub async fn health_check(
    service_core::axum::extract::State(state): service_core::axum::extract::State<AppState>,
) -> Result<service_core::axum::Json<serde_json::Value>, AppError> {
    // Check MongoDB connection
    state.db.health_check().await.map_err(|e| {
        tracing::error!(error = %e, "MongoDB health check failed");
        e
    })?;

    // Check Redis connection
    state.redis.health_check().await.map_err(|e| {
        tracing::error!(error = %e, "Redis health check failed");
        AppError::InternalError(e)
    })?;

    Ok(service_core::axum::Json(serde_json::json!({
        "status": "healthy",
        "service": state.config.service_name,
        "version": state.config.service_version,
        "environment": format!("{:?}", state.config.environment),
        "checks": {
            "mongodb": "up",
            "redis": "up"
        }
    })))
}
