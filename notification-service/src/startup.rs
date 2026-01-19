use crate::config::NotificationConfig;
use crate::handlers;
use crate::services::{
    EmailProvider, FcmProvider, MockEmailProvider, MockPushProvider, MockSmsProvider,
    Msg91Provider, NotificationDb, PushProvider, SmsProvider, SmtpProvider,
};
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
    pub config: NotificationConfig,
    pub db: NotificationDb,
    pub email_provider: Arc<dyn EmailProvider>,
    pub sms_provider: Arc<dyn SmsProvider>,
    pub push_provider: Arc<dyn PushProvider>,
}

pub struct Application {
    port: u16,
    server: Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + Unpin>,
    state: AppState,
}

impl Application {
    pub async fn build(config: NotificationConfig) -> Result<Self, AppError> {
        // Connect to database
        let db = NotificationDb::connect(&config.mongodb.uri, &config.mongodb.database)
            .await
            .map_err(|e| {
                tracing::error!("Failed to connect to MongoDB: {}", e);
                e
            })?;

        db.initialize_indexes().await.map_err(|e| {
            tracing::error!("Failed to initialize database indexes: {}", e);
            e
        })?;

        // Initialize providers
        let email_provider: Arc<dyn EmailProvider> = if config.smtp.enabled {
            match SmtpProvider::new(config.smtp.clone()) {
                Ok(provider) => {
                    tracing::info!("SMTP email provider initialized");
                    Arc::new(provider)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize SMTP provider: {}. Using mock.", e);
                    Arc::new(MockEmailProvider::new(true))
                }
            }
        } else {
            tracing::info!("SMTP provider disabled, using mock email provider");
            Arc::new(MockEmailProvider::new(true))
        };

        let sms_provider: Arc<dyn SmsProvider> = if config.msg91.enabled {
            tracing::info!("Msg91 SMS provider initialized");
            Arc::new(Msg91Provider::new(config.msg91.clone()))
        } else {
            tracing::info!("Msg91 provider disabled, using mock SMS provider");
            Arc::new(MockSmsProvider::new(true))
        };

        let push_provider: Arc<dyn PushProvider> = if config.fcm.enabled {
            tracing::info!("FCM push provider initialized");
            Arc::new(FcmProvider::new(config.fcm.clone()))
        } else {
            tracing::info!("FCM provider disabled, using mock push provider");
            Arc::new(MockPushProvider::new(true))
        };

        let state = AppState {
            config: config.clone(),
            db,
            email_provider,
            sms_provider,
            push_provider,
        };

        let app = Router::new()
            .route("/health", get(handlers::health_check))
            .route("/notifications/email", post(handlers::send_email))
            .route("/notifications/sms", post(handlers::send_sms))
            .route("/notifications/push", post(handlers::send_push))
            .route("/notifications/batch", post(handlers::send_batch))
            .route("/notifications/:id", get(handlers::get_notification))
            .route("/notifications", get(handlers::list_notifications))
            .layer(TraceLayer::new_for_http())
            .with_state(state.clone());

        let addr = SocketAddr::from(([0, 0, 0, 0], config.common.port));
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind TCP listener to {}: {}", addr, e);
            AppError::from(e)
        })?;
        let port = listener.local_addr()?.port();

        tracing::info!("Notification service listening on port {}", port);

        let server = axum::serve(listener, app);

        Ok(Self {
            port,
            server: Box::new(server.into_future()),
            state,
        })
    }

    pub fn db(&self) -> &NotificationDb {
        &self.state.db
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        self.server.await
    }
}
