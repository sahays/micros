use crate::config::DocumentConfig;
use crate::handlers;
use axum::{routing::get, Router};
use service_core::error::AppError;
use std::future::IntoFuture;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

pub struct Application {
    port: u16,
    server: Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + Unpin>,
}

impl Application {
    pub async fn build(config: DocumentConfig) -> Result<Self, AppError> {
        let app = Router::new()
            .route("/health", get(handlers::health_check))
            .layer(TraceLayer::new_for_http());

        let addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr()?.port();

        tracing::info!("Listening on {}", port);

        let server = axum::serve(listener, app);

        Ok(Self {
            port,
            server: Box::new(server.into_future()),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        self.server.await
    }
}
