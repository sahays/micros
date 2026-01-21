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
    /// Model for AUDIO output (e.g., gemini-2.0-flash with audio)
    pub audio_model: String,
    /// Model for VIDEO output (e.g., veo-2)
    pub video_model: String,
    /// Default content size threshold in bytes
    pub default_content_threshold_bytes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleConfig {
    pub api_key: String,
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
                text_model: get_env("GENAI_TEXT_MODEL", Some("gemini-2.0-flash"), is_prod)?,
                audio_model: get_env("GENAI_AUDIO_MODEL", Some("gemini-2.0-flash"), is_prod)?,
                video_model: get_env("GENAI_VIDEO_MODEL", Some("veo-2"), is_prod)?,
                default_content_threshold_bytes: get_env(
                    "GENAI_DEFAULT_CONTENT_THRESHOLD_BYTES",
                    Some(&DEFAULT_CONTENT_THRESHOLD_BYTES.to_string()),
                    is_prod,
                )?
                .parse()
                .unwrap_or(DEFAULT_CONTENT_THRESHOLD_BYTES),
            },
            google: GoogleConfig {
                api_key: get_env("GOOGLE_API_KEY", None, is_prod)?,
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
    pub fn model_for_output(&self, output_format: OutputFormat) -> &str {
        match output_format {
            OutputFormat::Text | OutputFormat::StructuredJson => &self.models.text_model,
            OutputFormat::Audio => &self.models.audio_model,
            OutputFormat::Video => &self.models.video_model,
        }
    }
}

/// Output format enum matching proto definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    StructuredJson,
    Audio,
    Video,
}

impl From<i32> for OutputFormat {
    fn from(value: i32) -> Self {
        match value {
            1 => OutputFormat::Text,
            2 => OutputFormat::StructuredJson,
            3 => OutputFormat::Audio,
            4 => OutputFormat::Video,
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
