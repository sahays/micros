use crate::config::DocumentConfig;
use crate::handlers;
use crate::services::{LocalStorage, MongoDb, Storage};
use axum::{
    routing::{get, post},
    Router,
};
use service_core::error::AppError;
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

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
        let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect to MongoDB: {}", e);
                e
            })?;
        db.initialize_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            e
        })?;

        let storage: Arc<dyn Storage> = Arc::new(
            LocalStorage::new(&config.storage.local_path)
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to initialize local storage at {}: {}",
                        config.storage.local_path,
                        e
                    );
                    e
                })?,
        );

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
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind TCP listener to {}: {}", addr, e);
            AppError::from(e)
        })?;
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
