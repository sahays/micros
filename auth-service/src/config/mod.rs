use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub environment: Environment,
    pub service_name: String,
    pub service_version: String,
    pub log_level: String,
    pub mongodb: MongoConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub google: GoogleOAuthConfig,
    pub gmail: GmailConfig,
    pub security: SecurityConfig,
    pub swagger: SwaggerConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Dev,
    Prod,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub private_key_path: String,
    pub public_key_path: String,
    pub access_token_expiry_minutes: i64,
    pub refresh_token_expiry_days: i64,
    pub app_token_expiry_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub frontend_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GmailConfig {
    pub user: String,
    pub app_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub allowed_origins: Vec<String>,
    pub require_signatures: bool,
    pub admin_api_key: String,
    #[serde(skip, default)]
    pub signature_config: service_core::middleware::signature::SignatureConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwaggerConfig {
    pub enabled: SwaggerMode,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SwaggerMode {
    Public,
    Authenticated,
    Disabled,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub login_attempts: u32,
    pub login_window_seconds: u64,
    pub register_attempts: u32,
    pub register_window_seconds: u64,
    pub password_reset_attempts: u32,
    pub password_reset_window_seconds: u64,
    pub global_ip_limit: u32,
    pub global_ip_window_seconds: u64,
    pub app_token_limit: u32,
    pub app_token_window_seconds: u64,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let common_config = core_config::Config::load()?;

        let env_str = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
        let environment: Environment = env_str
            .parse()
            .map_err(|e: String| AppError::ConfigError(anyhow::anyhow!(e)))?;

        let is_prod = environment == Environment::Prod;

        let config = AuthConfig {
            common: common_config,
            environment: environment.clone(),
            service_name: get_env("SERVICE_NAME", Some("auth-service"), is_prod)?,
            service_version: get_env("SERVICE_VERSION", Some(env!("CARGO_PKG_VERSION")), is_prod)?,
            log_level: get_env("LOG_LEVEL", Some("info"), is_prod)?,
            mongodb: MongoConfig {
                uri: get_env("MONGODB_URI", None, is_prod)?,
                database: get_env("MONGODB_DATABASE", None, is_prod)?,
            },
            redis: RedisConfig {
                url: get_env("REDIS_URL", None, is_prod)?,
            },
            jwt: JwtConfig {
                private_key_path: get_env("JWT_PRIVATE_KEY_PATH", None, is_prod)?,
                public_key_path: get_env("JWT_PUBLIC_KEY_PATH", None, is_prod)?,
                access_token_expiry_minutes: get_env(
                    "JWT_ACCESS_TOKEN_EXPIRY_MINUTES",
                    Some("15"),
                    is_prod,
                )?
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    AppError::ConfigError(anyhow::anyhow!(e.to_string()))
                })?,
                refresh_token_expiry_days: get_env(
                    "JWT_REFRESH_TOKEN_EXPIRY_DAYS",
                    Some("7"),
                    is_prod,
                )?
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    AppError::ConfigError(anyhow::anyhow!(e.to_string()))
                })?,
                app_token_expiry_minutes: get_env(
                    "JWT_APP_TOKEN_EXPIRY_MINUTES",
                    Some("60"),
                    is_prod,
                )?
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    AppError::ConfigError(anyhow::anyhow!(e.to_string()))
                })?,
            },
            google: GoogleOAuthConfig {
                client_id: get_env("GOOGLE_CLIENT_ID", None, is_prod)?,
                client_secret: get_env("GOOGLE_CLIENT_SECRET", None, is_prod)?,
                redirect_uri: get_env("GOOGLE_REDIRECT_URI", None, is_prod)?,
                frontend_url: get_env(
                    "GOOGLE_FRONTEND_URL",
                    Some("http://localhost:3000"),
                    is_prod,
                )?,
            },
            gmail: GmailConfig {
                user: get_env("GMAIL_USER", None, is_prod)?,
                app_password: get_env("GMAIL_APP_PASSWORD", None, is_prod)?,
            },
            security: {
                let require_signatures = get_env("REQUIRE_SIGNATURES", Some("false"), is_prod)?
                    .parse()
                    .unwrap_or(false);
                SecurityConfig {
                    allowed_origins: get_env(
                        "ALLOWED_ORIGINS",
                        Some("http://localhost:3000"),
                        is_prod,
                    )?
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                    require_signatures,
                    admin_api_key: get_env("ADMIN_API_KEY", None, true)?,
                    signature_config: service_core::middleware::signature::SignatureConfig {
                        require_signatures,
                        excluded_paths: vec![
                            "/health".to_string(),
                            "/.well-known/jwks.json".to_string(),
                            "/auth/verify".to_string(),
                            "/auth/google".to_string(),
                        ],
                    },
                }
            },
            swagger: SwaggerConfig {
                enabled: get_env("ENABLE_SWAGGER", Some("public"), is_prod)?
                    .parse()
                    .map_err(|e: String| AppError::ConfigError(anyhow::anyhow!(e)))?,
            },
            rate_limit: RateLimitConfig {
                login_attempts: get_env("RATE_LIMIT_LOGIN_ATTEMPTS", Some("5"), is_prod)?
                    .parse()
                    .unwrap_or(5),
                login_window_seconds: get_env(
                    "RATE_LIMIT_LOGIN_WINDOW_SECONDS",
                    Some("900"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(900),
                register_attempts: get_env("RATE_LIMIT_REGISTER_ATTEMPTS", Some("3"), is_prod)?
                    .parse()
                    .unwrap_or(3),
                register_window_seconds: get_env(
                    "RATE_LIMIT_REGISTER_WINDOW_SECONDS",
                    Some("3600"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(3600),
                password_reset_attempts: get_env(
                    "RATE_LIMIT_PASSWORD_RESET_ATTEMPTS",
                    Some("3"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(3),
                password_reset_window_seconds: get_env(
                    "RATE_LIMIT_PASSWORD_RESET_WINDOW_SECONDS",
                    Some("3600"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(3600),
                global_ip_limit: get_env("RATE_LIMIT_GLOBAL_IP_LIMIT", Some("100"), is_prod)?
                    .parse()
                    .unwrap_or(100),
                global_ip_window_seconds: get_env(
                    "RATE_LIMIT_GLOBAL_IP_WINDOW_SECONDS",
                    Some("60"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(60),
                app_token_limit: get_env("RATE_LIMIT_APP_TOKEN_LIMIT", Some("10"), is_prod)?
                    .parse()
                    .unwrap_or(10),
                app_token_window_seconds: get_env(
                    "RATE_LIMIT_APP_TOKEN_WINDOW_SECONDS",
                    Some("60"),
                    is_prod,
                )?
                .parse()
                .unwrap_or(60),
            },
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), AppError> {
        // Validate configuration
        if self.common.port == 0 {
            return Err(AppError::ConfigError(anyhow::anyhow!(
                "PORT must be greater than 0"
            )));
        }

        if self.jwt.access_token_expiry_minutes <= 0 {
            return Err(AppError::ConfigError(anyhow::anyhow!(
                "JWT_ACCESS_TOKEN_EXPIRY_MINUTES must be positive"
            )));
        }

        if self.jwt.refresh_token_expiry_days <= 0 {
            return Err(AppError::ConfigError(anyhow::anyhow!(
                "JWT_REFRESH_TOKEN_EXPIRY_DAYS must be positive"
            )));
        }

        // In production, ensure stricter validation
        if self.environment == Environment::Prod {
            if self.security.allowed_origins.iter().any(|o| o == "*") {
                return Err(AppError::ConfigError(anyhow::anyhow!(
                    "Wildcard CORS origin not allowed in production"
                )));
            }

            if self.swagger.enabled == SwaggerMode::Public {
                tracing::error!("Swagger is publicly accessible in production - consider using 'authenticated' or 'disabled'");
            }
        }

        Ok(())
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

impl std::str::FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dev" => Ok(Environment::Dev),
            "prod" => Ok(Environment::Prod),
            _ => Err(format!("Invalid environment: {}", s)),
        }
    }
}

impl std::str::FromStr for SwaggerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "public" => Ok(SwaggerMode::Public),
            "authenticated" => Ok(SwaggerMode::Authenticated),
            "disabled" => Ok(SwaggerMode::Disabled),
            _ => Err(format!("Invalid swagger mode: {}", s)),
        }
    }
}
