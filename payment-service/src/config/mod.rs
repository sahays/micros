use serde::Deserialize;
use std::env;
use anyhow::Result;
use dotenvy::dotenv;
use secrecy::Secret;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub signature: ServiceSignatureConfig,
    pub service_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DatabaseConfig {
    pub url: Secret<String>,
    pub db_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RedisConfig {
    pub url: Secret<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServiceSignatureConfig {
    pub enabled: bool,
    pub secret: Secret<String>,
    pub expiry_seconds: usize,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        let host = env::var("PAYMENT_SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PAYMENT_SERVICE_PORT")
            .unwrap_or_else(|_| "3003".to_string())
            .parse()?;

        let db_url = env::var("PAYMENT_DATABASE_URL").expect("PAYMENT_DATABASE_URL must be set");
        let db_name = env::var("PAYMENT_DATABASE_NAME").unwrap_or_else(|_| "payment_db".to_string());

        let redis_url = env::var("PAYMENT_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let signature_secret = env::var("PAYMENT_SIGNATURE_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
        let signature_enabled = env::var("PAYMENT_SIGNATURE_ENABLED").unwrap_or_else(|_| "false".to_string()).parse().unwrap_or(false);

        Ok(Self {
            server: ServerConfig { host, port },
            database: DatabaseConfig { 
                url: Secret::new(db_url), 
                db_name 
            },
            redis: RedisConfig {
                url: Secret::new(redis_url),
            },
            signature: ServiceSignatureConfig {
                enabled: signature_enabled,
                secret: Secret::new(signature_secret),
                expiry_seconds: 300,
            },
            service_name: "payment-service".to_string(),
        })
    }
}
