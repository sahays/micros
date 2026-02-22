use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub mongodb: MongoConfig,
    pub gmail: GmailApiConfig,
    pub msg91: Msg91Config,
    pub fcm: FcmConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GmailApiConfig {
    pub service_account_key_path: String,
    pub sender_email: String,
    pub sender_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Msg91Config {
    pub auth_key: String,
    pub sender_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FcmConfig {
    pub project_id: String,
    pub service_account_key: String,
    pub enabled: bool,
}

impl NotificationConfig {
    pub fn load() -> Result<Self, AppError> {
        let common_config = core_config::Config::load()?;
        let is_prod = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()) == "prod";

        Ok(NotificationConfig {
            common: common_config,
            mongodb: MongoConfig {
                uri: get_env("MONGODB_URI", None, is_prod)?,
                database: get_env("MONGODB_DATABASE", Some("notification_db"), is_prod)?,
            },
            gmail: GmailApiConfig {
                service_account_key_path: get_env(
                    "GOOGLE_SERVICE_ACCOUNT_KEY_PATH",
                    Some(""),
                    is_prod,
                )?,
                sender_email: get_env("GMAIL_API_SENDER_EMAIL", Some(""), is_prod)?,
                sender_name: get_env(
                    "GMAIL_API_SENDER_NAME",
                    Some("Notification Service"),
                    is_prod,
                )?,
                enabled: env::var("GMAIL_API_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            msg91: Msg91Config {
                auth_key: get_env("MSG91_AUTH_KEY", Some(""), is_prod)?,
                sender_id: get_env("MSG91_SENDER_ID", Some(""), is_prod)?,
                enabled: env::var("MSG91_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            fcm: FcmConfig {
                project_id: get_env("FCM_PROJECT_ID", Some(""), is_prod)?,
                service_account_key: get_env("FCM_SERVICE_ACCOUNT_KEY", Some(""), is_prod)?,
                enabled: env::var("FCM_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
        })
    }
}

fn get_env(key: &str, default: Option<&str>, is_prod: bool) -> Result<String, AppError> {
    match env::var(key) {
        Ok(val) => Ok(val),
        Err(_) => {
            if is_prod {
                Err(AppError::ConfigError(anyhow::anyhow!(
                    "{} is required in production but not set",
                    key
                )))
            } else if let Some(def) = default {
                Ok(def.to_string())
            } else {
                Err(AppError::ConfigError(anyhow::anyhow!(
                    "{} is required but not set",
                    key
                )))
            }
        }
    }
}
