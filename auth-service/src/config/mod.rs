use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub environment: Environment,
    pub service_name: String,
    pub service_version: String,
    pub log_level: String,
    pub port: u16,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
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
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        // Load .env file if it exists (for local development)
        dotenvy::dotenv().ok();

        let config = Config {
            environment: env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "dev".to_string())
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid ENVIRONMENT value"))?,
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "auth-service".to_string()),
            service_version: env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            log_level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid PORT value"))?,
            mongodb: MongoConfig {
                uri: env::var("MONGODB_URI")
                    .map_err(|_| anyhow::anyhow!("MONGODB_URI is required"))?,
                database: env::var("MONGODB_DATABASE")
                    .map_err(|_| anyhow::anyhow!("MONGODB_DATABASE is required"))?,
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL")
                    .map_err(|_| anyhow::anyhow!("REDIS_URL is required"))?,
            },
            jwt: JwtConfig {
                private_key_path: env::var("JWT_PRIVATE_KEY_PATH")
                    .map_err(|_| anyhow::anyhow!("JWT_PRIVATE_KEY_PATH is required"))?,
                public_key_path: env::var("JWT_PUBLIC_KEY_PATH")
                    .map_err(|_| anyhow::anyhow!("JWT_PUBLIC_KEY_PATH is required"))?,
                access_token_expiry_minutes: env::var("JWT_ACCESS_TOKEN_EXPIRY_MINUTES")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid JWT_ACCESS_TOKEN_EXPIRY_MINUTES"))?,
                refresh_token_expiry_days: env::var("JWT_REFRESH_TOKEN_EXPIRY_DAYS")
                    .unwrap_or_else(|_| "7".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid JWT_REFRESH_TOKEN_EXPIRY_DAYS"))?,
            },
            google: GoogleOAuthConfig {
                client_id: env::var("GOOGLE_CLIENT_ID")
                    .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_ID is required"))?,
                client_secret: env::var("GOOGLE_CLIENT_SECRET")
                    .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_SECRET is required"))?,
                redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                    .map_err(|_| anyhow::anyhow!("GOOGLE_REDIRECT_URI is required"))?,
            },
            gmail: GmailConfig {
                user: env::var("GMAIL_USER")
                    .map_err(|_| anyhow::anyhow!("GMAIL_USER is required"))?,
                app_password: env::var("GMAIL_APP_PASSWORD")
                    .map_err(|_| anyhow::anyhow!("GMAIL_APP_PASSWORD is required"))?,
            },
            security: SecurityConfig {
                allowed_origins: env::var("ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                require_signatures: env::var("REQUIRE_SIGNATURES")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            swagger: SwaggerConfig {
                enabled: env::var("ENABLE_SWAGGER")
                    .unwrap_or_else(|_| "public".to_string())
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid ENABLE_SWAGGER value"))?,
            },
            rate_limit: RateLimitConfig {
                login_attempts: env::var("RATE_LIMIT_LOGIN_ATTEMPTS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()
                    .unwrap_or(5),
                login_window_seconds: env::var("RATE_LIMIT_LOGIN_WINDOW_SECONDS")
                    .unwrap_or_else(|_| "900".to_string())
                    .parse()
                    .unwrap_or(900),
                register_attempts: env::var("RATE_LIMIT_REGISTER_ATTEMPTS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                register_window_seconds: env::var("RATE_LIMIT_REGISTER_WINDOW_SECONDS")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
                password_reset_attempts: env::var("RATE_LIMIT_PASSWORD_RESET_ATTEMPTS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                password_reset_window_seconds: env::var("RATE_LIMIT_PASSWORD_RESET_WINDOW_SECONDS")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
            },
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), anyhow::Error> {
        // Validate configuration
        if self.port == 0 {
            return Err(anyhow::anyhow!("PORT must be greater than 0"));
        }

        if self.jwt.access_token_expiry_minutes <= 0 {
            return Err(anyhow::anyhow!("JWT_ACCESS_TOKEN_EXPIRY_MINUTES must be positive"));
        }

        if self.jwt.refresh_token_expiry_days <= 0 {
            return Err(anyhow::anyhow!("JWT_REFRESH_TOKEN_EXPIRY_DAYS must be positive"));
        }

        // In production, ensure stricter validation
        if self.environment == Environment::Prod {
            if self.security.allowed_origins.iter().any(|o| o == "*") {
                return Err(anyhow::anyhow!("Wildcard CORS origin not allowed in production"));
            }

            if self.swagger.enabled == SwaggerMode::Public {
                tracing::warn!("Swagger is publicly accessible in production - consider using 'authenticated' or 'disabled'");
            }
        }

        Ok(())
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
