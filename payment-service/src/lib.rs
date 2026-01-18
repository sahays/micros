pub mod config;
pub mod dtos;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod utils;

use axum::middleware::from_fn;
use axum::middleware::from_fn_with_state;
use axum::{
    routing::{get, post},
    Router,
};
use mongodb::{options::ClientOptions, Client};
use secrecy::ExposeSecret;
use service_core::middleware::{
    metrics::metrics_middleware,
    signature::{signature_validation_middleware, SignatureConfig, SignatureStore},
    tracing::request_id_middleware,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

use config::Config;
use services::{PaymentRepository, RazorpayClient};

#[derive(Clone)]
pub struct AppState {
    pub db: mongodb::Database,
    pub redis: redis::Client,
    pub config: Config,
    pub signature_config: SignatureConfig,
    pub repository: PaymentRepository,
    pub razorpay: RazorpayClient,
}

impl AsRef<SignatureConfig> for AppState {
    fn as_ref(&self) -> &SignatureConfig {
        &self.signature_config
    }
}

#[async_trait::async_trait]
impl SignatureStore for AppState {
    async fn validate_nonce(&self, nonce: &str) -> Result<bool, service_core::error::AppError> {
        let mut con = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                tracing::error!("Failed to get redis connection: {}", e);
                service_core::error::AppError::InternalError(anyhow::anyhow!("Redis error"))
            })?;

        let key = format!("nonce:{}", nonce);
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut con)
            .await
            .unwrap_or(false);

        if exists {
            return Ok(false);
        }

        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("EX")
            .arg(self.config.signature.expiry_seconds)
            .query_async(&mut con)
            .await
            .map_err(|e| {
                tracing::error!("Failed to set nonce: {}", e);
                service_core::error::AppError::InternalError(anyhow::anyhow!("Redis error"))
            })?;

        Ok(true)
    }

    async fn get_signing_secret(
        &self,
        _client_id: &str,
    ) -> Result<Option<String>, service_core::error::AppError> {
        // In a real scenario, we would lookup client credentials from DB.
        // For now, we return a hardcoded secret or one from config for "internal" clients.
        // This is a simplification for the prototype phase.
        Ok(Some(self.config.signature.secret.expose_secret().clone()))
    }
}

pub struct Application {
    port: u16,
    router: Router,
}

impl Application {
    pub async fn build(config: Config) -> anyhow::Result<Self> {
        let mut client_options = ClientOptions::parse(config.database.url.expose_secret()).await?;
        client_options.app_name = Some("payment-service".to_string());

        let client = Client::with_options(client_options)?;
        let db = client.database(&config.database.db_name);

        let redis = redis::Client::open(config.redis.url.expose_secret().as_str())?;

        let signature_config = SignatureConfig {
            require_signatures: config.signature.enabled,
            excluded_paths: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/docs".to_string(),
                "/.well-known/openapi.json".to_string(),
                "/webhooks/razorpay".to_string(), // Webhooks from Razorpay, not BFF
            ],
        };

        let repository = PaymentRepository::new(&db);

        // Initialize indexes for tenant-scoped queries
        repository.init_indexes().await?;

        // Initialize Razorpay client
        let razorpay = RazorpayClient::new(config.razorpay.clone());
        if razorpay.is_configured() {
            tracing::info!("Razorpay client initialized");
        } else {
            tracing::warn!(
                "Razorpay credentials not configured - payment features will be limited"
            );
        }

        let state = AppState {
            db,
            redis,
            config: config.clone(),
            signature_config,
            repository,
            razorpay,
        };

        let router = Router::new()
            .route("/health", get(handlers::health_check))
            .route("/metrics", get(handlers::metrics))
            .route("/payments/qr/generate", post(handlers::upi::generate_qr))
            // Transaction endpoints (tenant-scoped)
            .route(
                "/transactions",
                post(handlers::transactions::create_transaction),
            )
            .route(
                "/transactions/:id",
                get(handlers::transactions::get_transaction),
            )
            .route(
                "/transactions/:id/status",
                axum::routing::patch(handlers::transactions::update_transaction_status),
            )
            // Razorpay endpoints
            .route("/orders", post(handlers::razorpay::create_order))
            .route("/orders/:id", get(handlers::razorpay::get_transaction))
            .route("/payments/verify", post(handlers::razorpay::verify_payment))
            .route("/webhooks/razorpay", post(handlers::razorpay::webhook))
            .layer(from_fn_with_state(
                state.clone(),
                signature_validation_middleware::<AppState>,
            ))
            .layer(from_fn(metrics_middleware))
            .layer(from_fn(request_id_middleware))
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
            .with_state(state);

        Ok(Self {
            port: config.server.port,
            router,
        })
    }

    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        tracing::info!("Listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router).await?;

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
