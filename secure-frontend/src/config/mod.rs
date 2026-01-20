use secrecy::Secret;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub auth_service: AuthServiceSettings,
    pub document_service: DocumentServiceSettings,
}

#[derive(Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub session_secret: Secret<String>,
}

#[derive(Deserialize, Clone)]
pub struct AuthServiceSettings {
    /// HTTP URL for OAuth redirects (browser-accessible).
    pub url: String,
    /// URL accessible from browser for OAuth flows (e.g., localhost:9005).
    pub public_url: String,
    /// gRPC endpoint for internal service calls (e.g., http://auth-service:50051).
    #[serde(default = "default_auth_grpc_url")]
    pub grpc_url: String,
    /// Default tenant slug for BFF operations.
    #[serde(default = "default_tenant_slug")]
    pub default_tenant_slug: String,
}

fn default_auth_grpc_url() -> String {
    "http://localhost:50051".to_string()
}

fn default_tenant_slug() -> String {
    "default".to_string()
}

#[derive(Deserialize, Clone)]
pub struct DocumentServiceSettings {
    /// HTTP URL (kept for backward compatibility, may be removed).
    pub url: String,
    /// gRPC endpoint for internal service calls (e.g., http://document-service:8081).
    #[serde(default = "default_document_grpc_url")]
    pub grpc_url: String,
    /// Secret used for generating signed shareable URLs for documents.
    /// This is NOT for KYS authentication - it's for time-limited public download links.
    pub document_signing_secret: Secret<String>,
    /// Default app_id for document operations.
    #[serde(default = "default_app_id")]
    pub default_app_id: String,
    /// Default org_id for document operations.
    #[serde(default = "default_org_id")]
    pub default_org_id: String,
}

fn default_document_grpc_url() -> String {
    "http://localhost:50053".to_string()
}

fn default_app_id() -> String {
    "secure-frontend".to_string()
}

fn default_org_id() -> String {
    "default".to_string()
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");

    // Check if we're already in secure-frontend directory or need to navigate to it
    let configuration_directory = if base_path.ends_with("secure-frontend") {
        base_path.join("config")
    } else {
        base_path.join("secure-frontend").join("config")
    };

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base.yaml")).required(true))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
