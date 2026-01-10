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
    pub client_id: String,
    pub signing_secret: Secret<String>,
}

#[derive(Deserialize, Clone)]
pub struct DocumentServiceSettings {
    pub url: String,
    pub client_id: String,
    pub signing_secret: Secret<String>,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("secure-frontend").join("config");

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
