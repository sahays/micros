use anyhow::Result;
use dotenvy::dotenv;
use secrecy::Secret;
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub signature: ServiceSignatureConfig,
    pub upi: UpiConfig,
    pub razorpay: RazorpayConfig,
    pub service_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RazorpayConfig {
    pub key_id: String,
    pub key_secret: Secret<String>,
    pub webhook_secret: Secret<String>,
    pub api_base_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct UpiConfig {
    pub vpa: String,
    pub merchant_name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub grpc_port: u16,
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
        let grpc_port = env::var("PAYMENT_SERVICE_GRPC_PORT")
            .unwrap_or_else(|_| "3004".to_string())
            .parse()?;

        let db_url = env::var("PAYMENT_DATABASE_URL").expect("PAYMENT_DATABASE_URL must be set");
        let db_name =
            env::var("PAYMENT_DATABASE_NAME").unwrap_or_else(|_| "payment_db".to_string());

        let redis_url =
            env::var("PAYMENT_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let signature_secret =
            env::var("PAYMENT_SIGNATURE_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
        let signature_enabled = env::var("PAYMENT_SIGNATURE_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let upi_vpa = env::var("PAYMENT_UPI_VPA").unwrap_or_else(|_| "merchant@upi".to_string());
        let upi_merchant_name =
            env::var("PAYMENT_UPI_MERCHANT_NAME").unwrap_or_else(|_| "Micros Merchant".to_string());

        // Razorpay configuration
        let razorpay_key_id = env::var("RAZORPAY_KEY_ID").unwrap_or_else(|_| "".to_string());
        let razorpay_key_secret =
            env::var("RAZORPAY_KEY_SECRET").unwrap_or_else(|_| "".to_string());
        let razorpay_webhook_secret =
            env::var("RAZORPAY_WEBHOOK_SECRET").unwrap_or_else(|_| "".to_string());
        let razorpay_api_base_url = env::var("RAZORPAY_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.razorpay.com/v1".to_string());

        Ok(Self {
            server: ServerConfig { host, port, grpc_port },
            database: DatabaseConfig {
                url: Secret::new(db_url),
                db_name,
            },
            redis: RedisConfig {
                url: Secret::new(redis_url),
            },
            signature: ServiceSignatureConfig {
                enabled: signature_enabled,
                secret: Secret::new(signature_secret),
                expiry_seconds: 300,
            },
            upi: UpiConfig {
                vpa: upi_vpa,
                merchant_name: upi_merchant_name,
            },
            razorpay: RazorpayConfig {
                key_id: razorpay_key_id,
                key_secret: Secret::new(razorpay_key_secret),
                webhook_secret: Secret::new(razorpay_webhook_secret),
                api_base_url: razorpay_api_base_url,
            },
            service_name: "payment-service".to_string(),
        })
    }
}
