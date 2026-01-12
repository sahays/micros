use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub mongodb: MongoConfig,
    pub storage: StorageConfig,
    pub signature: SignatureConfig,
    pub worker: WorkerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignatureConfig {
    pub require_signatures: bool,
    pub signing_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub local_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerConfig {
    pub enabled: bool,
    pub worker_count: usize,
    pub queue_size: usize,
    pub command_timeout_seconds: u64,
    pub temp_dir: PathBuf,
}

impl WorkerConfig {
    pub fn command_timeout(&self) -> Duration {
        Duration::from_secs(self.command_timeout_seconds)
    }
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
                local_path: get_env("STORAGE_LOCAL_PATH", Some("storage"), is_prod)?,
            },
            signature: SignatureConfig {
                require_signatures: env::var("REQUIRE_SIGNATURES")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                signing_secret: get_env("SIGNING_SECRET", Some("dev-signing-secret"), is_prod)?,
            },
            worker: WorkerConfig {
                enabled: env::var("WORKER_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                worker_count: env::var("WORKER_COUNT")
                    .unwrap_or_else(|_| "4".to_string())
                    .parse()
                    .unwrap_or(4),
                queue_size: env::var("QUEUE_SIZE")
                    .unwrap_or_else(|_| "100".to_string())
                    .parse()
                    .unwrap_or(100),
                command_timeout_seconds: env::var("COMMAND_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
                temp_dir: PathBuf::from(get_env(
                    "TEMP_DIR",
                    Some("/tmp/document-processing"),
                    is_prod,
                )?),
            },
        })
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
