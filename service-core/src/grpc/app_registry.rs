//! Redis-backed app registry and gRPC handler.
//!
//! Apps register with a fixed `app_id` on startup. Each service validates
//! incoming `x-app-id` headers against Redis.

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::proto::common::app_registry_service_server::AppRegistryService;
use super::proto::common::{RegisterAppRequest, RegisterAppResponse};

/// Redis key prefix for app registrations.
const APP_KEY_PREFIX: &str = "app:";

/// Redis-backed app registry. Apps register with a fixed app_id on startup.
#[derive(Clone)]
pub struct AppRegistry {
    redis: ConnectionManager,
}

impl AppRegistry {
    /// Create a new AppRegistry connected to the given Redis URL.
    pub async fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        let client = redis::Client::open(redis_url)?;
        let manager = client.get_connection_manager().await?;
        Ok(Self { redis: manager })
    }

    /// Store an app registration. Idempotent (overwrites with same data).
    async fn register(&self, app_id: &str, app_label: &str) -> Result<(), anyhow::Error> {
        let key = format!("{}{}", APP_KEY_PREFIX, app_id);
        let mut conn = self.redis.clone();
        conn.set::<_, _, ()>(&key, app_label).await?;
        Ok(())
    }

    /// Check if app_id is registered.
    pub async fn is_valid(&self, app_id: &str) -> bool {
        let key = format!("{}{}", APP_KEY_PREFIX, app_id);
        let mut conn = self.redis.clone();
        conn.exists::<_, bool>(&key).await.unwrap_or(false)
    }

    /// Get the app_label for a registered app_id. Returns None if not found.
    pub async fn get_label(&self, app_id: &str) -> Option<String> {
        let key = format!("{}{}", APP_KEY_PREFIX, app_id);
        let mut conn = self.redis.clone();
        conn.get::<_, Option<String>>(&key).await.ok().flatten()
    }

    /// Extract x-app-id from gRPC metadata and validate against Redis.
    pub async fn require_app_id<T>(&self, request: &Request<T>) -> Result<String, Status> {
        let app_id = request
            .metadata()
            .get(super::interceptors::APP_ID_KEY)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| Status::unauthenticated("missing x-app-id header"))?;

        if !self.is_valid(&app_id).await {
            return Err(Status::permission_denied(format!(
                "unknown app_id: {}",
                app_id
            )));
        }

        Ok(app_id)
    }
}

/// gRPC handler for the AppRegistryService.
pub struct AppRegistryServiceImpl {
    registry: Arc<AppRegistry>,
}

impl AppRegistryServiceImpl {
    pub fn new(registry: Arc<AppRegistry>) -> Self {
        Self { registry }
    }
}

#[tonic::async_trait]
impl AppRegistryService for AppRegistryServiceImpl {
    async fn register_app(
        &self,
        request: Request<RegisterAppRequest>,
    ) -> Result<Response<RegisterAppResponse>, Status> {
        let req = request.into_inner();

        // Validate app_id: non-empty, 1-63 chars, lowercase alphanumeric + hyphens
        if req.app_id.is_empty() || req.app_id.len() > 63 {
            return Err(Status::invalid_argument(
                "app_id must be 1-63 characters",
            ));
        }
        if !req
            .app_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(Status::invalid_argument(
                "app_id must contain only lowercase alphanumeric characters and hyphens",
            ));
        }
        if req.app_id.starts_with('-') || req.app_id.ends_with('-') {
            return Err(Status::invalid_argument(
                "app_id must not start or end with a hyphen",
            ));
        }

        // Validate app_label
        let app_label = if req.app_label.is_empty() {
            req.app_id.clone()
        } else {
            req.app_label
        };

        self.registry
            .register(&req.app_id, &app_label)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, app_id = %req.app_id, "Failed to register app");
                Status::internal("failed to register app")
            })?;

        tracing::info!(app_id = %req.app_id, app_label = %app_label, "App registered");

        Ok(Response::new(RegisterAppResponse {
            app_id: req.app_id,
            app_label,
        }))
    }
}
