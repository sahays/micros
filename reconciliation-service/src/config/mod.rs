//! Configuration module for reconciliation-service.

use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone)]
pub struct ReconciliationConfig {
    pub common: core_config::Config,
    pub service_name: String,
    pub service_version: String,
    pub log_level: String,
    pub otlp_endpoint: Option<String>,
    pub database: DatabaseConfig,
    pub ledger_service: LedgerServiceConfig,
    pub genai_service: GenaiServiceConfig,
    pub document_service: DocumentServiceConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone)]
pub struct LedgerServiceConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct GenaiServiceConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct DocumentServiceConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub auth_service_endpoint: String,
}

impl ReconciliationConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let common = core_config::Config::load()?;

        Ok(Self {
            common,
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "reconciliation-service".to_string()),
            service_version: env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            otlp_endpoint: env::var("OTLP_ENDPOINT").ok(),
            database: DatabaseConfig {
                url: env::var("DATABASE_URL").map_err(|_| {
                    AppError::ConfigError(anyhow::anyhow!("DATABASE_URL is required"))
                })?,
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2),
            },
            ledger_service: LedgerServiceConfig {
                url: env::var("LEDGER_SERVICE_URL")
                    .unwrap_or_else(|_| "http://ledger-service:3001".to_string()),
            },
            genai_service: GenaiServiceConfig {
                url: env::var("GENAI_SERVICE_URL")
                    .unwrap_or_else(|_| "http://genai-service:3001".to_string()),
            },
            document_service: DocumentServiceConfig {
                url: env::var("DOCUMENT_SERVICE_URL")
                    .unwrap_or_else(|_| "http://document-service:3001".to_string()),
            },
            auth: AuthConfig {
                auth_service_endpoint: env::var("AUTH_SERVICE_ENDPOINT")
                    .unwrap_or_else(|_| "http://auth-service:3001".to_string()),
            },
        })
    }
}
