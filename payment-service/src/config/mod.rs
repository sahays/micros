use serde::Deserialize;
use std::env;
use anyhow::Result;
use dotenvy::dotenv;
use secrecy::Secret;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
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

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        let host = env::var("PAYMENT_SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PAYMENT_SERVICE_PORT")
            .unwrap_or_else(|_| "3003".to_string())
            .parse()?;

        let db_url = env::var("PAYMENT_DATABASE_URL").expect("PAYMENT_DATABASE_URL must be set");
        let db_name = env::var("PAYMENT_DATABASE_NAME").unwrap_or_else(|_| "payment_db".to_string());

        Ok(Self {
            server: ServerConfig { host, port },
            database: DatabaseConfig { 
                url: Secret::new(db_url), 
                db_name 
            },
        })
    }
}
