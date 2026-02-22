use serde::Deserialize;
use service_core::config as core_config;
use service_core::error::AppError;
use std::env;

/// Default content size threshold (1MB) - if document content exceeds this,
/// service uses maximum token output.
const DEFAULT_CONTENT_THRESHOLD_BYTES: i64 = 1_048_576;

#[derive(Debug, Clone, Deserialize)]
pub struct GenaiConfig {
    #[serde(flatten)]
    pub common: core_config::Config,
    pub mongodb: MongoConfig,
    pub models: ModelConfig,
    pub google: GoogleConfig,
    pub document_service: DocumentServiceConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// Model for TEXT and STRUCTURED_JSON output (e.g., gemini-2.0-flash)
    pub text_model: String,
    /// Default content size threshold in bytes
    pub default_content_threshold_bytes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleConfig {
    pub service_account_key_path: String,
    /// Region for Gemini text models (e.g., "global", "us-central1").
    pub gemini_region: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentServiceConfig {
    pub grpc_url: String,
}

impl GenaiConfig {
    pub fn load() -> Result<Self, AppError> {
        let common_config = core_config::Config::load()?;
        let is_prod = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()) == "prod";

        Ok(GenaiConfig {
            common: common_config,
            mongodb: MongoConfig {
                uri: get_env("MONGODB_URI", None, is_prod)?,
                database: get_env("MONGODB_DATABASE", Some("genai_db"), is_prod)?,
            },
            models: ModelConfig {
                text_model: get_env("GENAI_TEXT_MODEL", Some("gemini-3-flash-preview"), is_prod)?,
                default_content_threshold_bytes: {
                    let raw = get_env(
                        "GENAI_DEFAULT_CONTENT_THRESHOLD_BYTES",
                        Some(&DEFAULT_CONTENT_THRESHOLD_BYTES.to_string()),
                        is_prod,
                    )?;
                    match raw.parse() {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(
                                key = "GENAI_DEFAULT_CONTENT_THRESHOLD_BYTES",
                                value = %raw,
                                error = %e,
                                default = DEFAULT_CONTENT_THRESHOLD_BYTES,
                                "Failed to parse config value, using default"
                            );
                            DEFAULT_CONTENT_THRESHOLD_BYTES
                        }
                    }
                },
            },
            google: GoogleConfig {
                service_account_key_path: get_env(
                    "GOOGLE_SERVICE_ACCOUNT_KEY_PATH",
                    None,
                    is_prod,
                )?,
                gemini_region: get_env("GEMINI_REGION", Some("global"), is_prod)?,
            },
            document_service: DocumentServiceConfig {
                grpc_url: get_env(
                    "DOCUMENT_SERVICE_GRPC_URL",
                    Some("http://document-service:8081"),
                    is_prod,
                )?,
            },
        })
    }

    /// Get the appropriate model based on output format.
    pub fn model_for_output(&self, _output_format: OutputFormat) -> &str {
        &self.models.text_model
    }

    /// Check whether a model string matches one of the configured models.
    pub fn is_valid_model(&self, model: &str) -> bool {
        model == self.models.text_model
    }

    /// Return the list of configured model names.
    pub fn valid_models(&self) -> Vec<&str> {
        vec![self.models.text_model.as_str()]
    }
}

/// Output format enum matching proto definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    StructuredJson,
}

impl From<i32> for OutputFormat {
    fn from(value: i32) -> Self {
        match value {
            2 => OutputFormat::StructuredJson,
            _ => OutputFormat::Text, // Default to text for unspecified
        }
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
