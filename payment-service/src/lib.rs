pub mod config;
pub mod handlers;
pub mod models;
pub mod dtos;
pub mod middleware;
pub mod services;
pub mod utils;

use axum::{routing::get, Router};
use mongodb::{options::ClientOptions, Client};
use secrecy::ExposeSecret;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: mongodb::Database,
    pub config: Config,
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

        let state = AppState {
            db,
            config: config.clone(),
        };

        let router = Router::new()
            .route("/health", get(handlers::health_check))
            .layer(TraceLayer::new_for_http())
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
