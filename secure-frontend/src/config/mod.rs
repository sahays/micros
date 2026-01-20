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
    pub url: String,
    pub public_url: String, // URL accessible from browser (e.g., localhost:9005)
}

#[derive(Deserialize, Clone)]
pub struct DocumentServiceSettings {
    pub url: String,
    /// Secret used for generating signed shareable URLs for documents.
    /// This is NOT for KYS authentication - it's for time-limited public download links.
    pub document_signing_secret: Secret<String>,
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
