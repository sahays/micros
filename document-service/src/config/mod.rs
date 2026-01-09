use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
}

impl DocumentConfig {
    pub fn load() -> Result<Self, AppError> {
        // Load common config (handles .env and APP__ prefix)
        let common_config = core_config::Config::load()?;

        // For now, we just wrap the common config.
        // As the service grows, we will add more fields here and load them from env.

        Ok(DocumentConfig {
            common: common_config,
        })
    }
}
