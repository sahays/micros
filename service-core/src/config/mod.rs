use crate::error::AppError;
use config::{Config as Cfg, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    8080
}

impl Config {
    pub fn load() -> Result<Self, AppError> {
        dotenvy::dotenv().ok();

        let config = Cfg::builder()
            .add_source(File::with_name("configuration").required(false))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}
