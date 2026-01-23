use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub mongodb: MongoConfig,
    pub smtp: SmtpConfig,
    pub msg91: Msg91Config,
    pub fcm: FcmConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// When set, enables capability enforcement via auth-service.
    /// Leave empty to use BFF trust model.
    pub auth_service_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
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
            smtp: SmtpConfig {
                host: get_env("SMTP_HOST", Some("smtp.gmail.com"), is_prod)?,
                port: get_env("SMTP_PORT", Some("587"), is_prod)?
                    .parse()
                    .unwrap_or(587),
                user: get_env("SMTP_USER", Some(""), is_prod)?,
                password: get_env("SMTP_PASSWORD", Some(""), is_prod)?,
                from_email: get_env("SMTP_FROM_EMAIL", Some("noreply@example.com"), is_prod)?,
                from_name: get_env("SMTP_FROM_NAME", Some("Notification Service"), is_prod)?,
                enabled: env::var("SMTP_ENABLED")
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
            auth: AuthConfig {
                // When set, capability enforcement is enabled via auth-service.
                // Leave empty/unset for BFF trust model (default).
                auth_service_endpoint: env::var("AUTH_SERVICE_ENDPOINT").ok(),
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
