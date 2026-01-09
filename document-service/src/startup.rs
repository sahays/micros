use crate::config::{DocumentConfig, StorageBackend};
use crate::handlers;
use crate::services::{MongoDb, Storage, LocalStorage, S3Storage};
use axum::{Router, routing::{get, post}};
use service_core::error::AppError;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use std::future::IntoFuture;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: DocumentConfig,
    pub db: MongoDb,
    pub storage: Arc<dyn Storage>,
}

pub struct Application {
    port: u16,
    server: Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + Unpin>,
    state: AppState,
}

impl Application {
    pub async fn build(config: DocumentConfig) -> Result<Self, AppError> {
        let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database).await?;
        db.initialize_indexes().await?;

        let storage: Arc<dyn Storage> = match config.storage.backend {
            StorageBackend::Local => {
                let path = config.storage.local_path.as_deref().unwrap_or("storage");
                Arc::new(LocalStorage::new(path).await?)
            }
            StorageBackend::S3 => {
                let s3_config = aws_config::load_from_env().await;
                let client = aws_sdk_s3::Client::new(&s3_config);
                let bucket = config.storage.s3_bucket.clone().ok_or_else(|| {
                    AppError::ConfigError(anyhow::anyhow!("STORAGE_S3_BUCKET is required for S3 backend"))
                })?;
                Arc::new(S3Storage::new(client, bucket))
            }
        };

        let state = AppState {
            config: config.clone(),
            db: db.clone(),
            storage,
        };

        let app = Router::new()
            .route("/health", get(handlers::health_check))
            .route("/documents", post(handlers::upload_document))
            .layer(TraceLayer::new_for_http())
            .with_state(state.clone());

        let addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr()?.port();

        tracing::info!("Listening on {}", port);

        let server = axum::serve(listener, app);

        Ok(Self {
            port,
            server: Box::new(server.into_future()),
            state,
        })
    }

    pub fn db(&self) -> &MongoDb {
        &self.state.db
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        self.server.await
    }
}

