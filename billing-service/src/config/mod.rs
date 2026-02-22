//! Configuration module for billing-service.

use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone)]
pub struct BillingConfig {
    pub common: core_config::Config,
    pub service_name: String,
    pub service_version: String,
    pub log_level: String,
    pub database: DatabaseConfig,
    pub invoicing_service: InvoicingServiceConfig,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone)]
pub struct InvoicingServiceConfig {
    pub url: String,
}

impl BillingConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let common = core_config::Config::load()?;

        Ok(Self {
            common,
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "billing-service".to_string()),
            service_version: env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
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
            invoicing_service: InvoicingServiceConfig {
                url: env::var("INVOICING_SERVICE_URL")
                    .unwrap_or_else(|_| "http://invoicing-service:3001".to_string()),
            },
        })
    }
}
