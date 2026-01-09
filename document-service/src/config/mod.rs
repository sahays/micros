use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub mongodb: MongoConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub local_path: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Local,
    S3,
}

impl DocumentConfig {
    pub fn load() -> Result<Self, AppError> {
        // Load common config (handles .env and APP__ prefix)
        let common_config = core_config::Config::load()?;

        // For now, we wrap the common config and add MongoDB and Storage config.
        let is_prod = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()) == "prod";

        Ok(DocumentConfig {
            common: common_config,
            mongodb: MongoConfig {
                uri: get_env("MONGODB_URI", None, is_prod)?,
                database: get_env("MONGODB_DATABASE", Some("document_db"), is_prod)?,
            },
            storage: StorageConfig {
                backend: get_env("STORAGE_BACKEND", Some("local"), is_prod)?
                    .parse()
                    .map_err(|e: String| AppError::ConfigError(anyhow::anyhow!(e)))?,
                local_path: Some(get_env("STORAGE_LOCAL_PATH", Some("storage"), is_prod)?),
                s3_bucket: env::var("STORAGE_S3_BUCKET").ok(),
                s3_region: env::var("STORAGE_S3_REGION").ok(),
            },
        })
    }
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(StorageBackend::Local),
            "s3" => Ok(StorageBackend::S3),
            _ => Err(format!("Invalid storage backend: {}", s)),
        }
    }
}

fn get_env(key: &str, default: Option<&str>, is_prod: bool) -> Result<String, AppError> {
    match env::var(key) {
        Ok(val) => Ok(val),
        Err(_) => {
            if is_prod {
                Err(AppError::ConfigError(anyhow::anyhow!(format!(
                    "{} is required in production but not set",
                    key
                ))))
            } else if let Some(def) = default {
                Ok(def.to_string())
            } else {
                Err(AppError::ConfigError(anyhow::anyhow!(format!(
                    "{} is required but not set",
                    key
                ))))
            }
        }
    }
}
